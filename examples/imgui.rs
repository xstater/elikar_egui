use elikar::{Elikar, States, common::Spawner, window::{Window, events::WindowEventType}};
use futures::{StreamExt, executor::block_on};
use xecs::{query::WithId, system::System};

fn main() {
    let mut game = Elikar::new().unwrap();
    let world = game.world();
    
    let window_id = game.window_builder()
        .title("imgui")
        .size(1280, 800)
        .always_on_top()
        .resizable()
        .vulkan()
        .build()
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);
    let surface = {
        let world = world.read().unwrap();
        let window = world.query::<&Window>().with_id()
            .find(|(id,_)|*id == window_id)
            .map(|(_,window)|window)
            .unwrap();
        unsafe { instance.create_surface(&window) }
    };

    let adapter = block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions{
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor{
                label: Some("device"),
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },None))
    .unwrap();

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width : 1280,
        height : 800,
        present_mode: wgpu::PresentMode::Immediate,
    };
    surface.configure(&device, &config);

    //store these in world
    {
        let mut world = world.write().unwrap();
        world.store_resource(device);
        world.store_resource(queue);
        world.store_resource(config);
        world.store_resource(surface);
    }

    let events = game.events();
    game.spawn_local(async move{
        let mut quit = events.on_quit();
        let world = quit.world();
        if let Some(_) = quit.next().await {
            let world = world.read().unwrap();
            let mut states = world.resource_mut::<States>().unwrap();
            states.quit()
        }
    });

    let events = game.events();
    game.spawn_local(async move {
        let mut window_event = events.on_window_events();
        let world = window_event.world();
        while let Some(window) = window_event.next().await {
            let size = match window.event_type {
                WindowEventType::Resized(w, h) => Some((w,h)),
                WindowEventType::SizeChanged(w, h) => Some((w,h)),
                _ => None
            };

            if let Some((w,h)) = size {
                if w == 0 || h == 0 { continue; }

                let world = world.read().unwrap();
                let surface = world.resource_ref::<wgpu::Surface>().unwrap();
                let device = world.resource_ref::<wgpu::Device>().unwrap();
                let mut config = world.resource_mut::<wgpu::SurfaceConfiguration>().unwrap();
                config.width = w;
                config.height = h;
                surface.configure(&device, &config)
            }
        }
    });

    let events = game.events();
    let prepared = elikar_egui::build(&mut game, events);

    game.spawn_local(async move{
        let mut prepared = prepared;
        let world = prepared.world();

        let mut name = String::new();
        let mut age = 18;
        while let Some(ctx_ref) = prepared.next().await {
            egui::CentralPanel::default()
                .show(&ctx_ref,|ui|{
                    ui.heading("My egui Application");
                    ui.horizontal(|ui| {
                        ui.label("Your name: ");
                        ui.text_edit_singleline(&mut name);
                    });
                    ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
                    if ui.button("Click each year").clicked() {
                        dbg!("Added");
                        age += 1;
                    }
                    ui.label(format!("Hello '{}', age {}", name, age));
                    let fps = {
                        let world = world.read().unwrap();
                        let states = world.resource_ref::<States>().unwrap();
                        states.actual_fps()
                    };
                    ui.label(format!("Hello '{}', age {}", name, age));
                    ui.label(format!("Fps : {} Hz",fps));
                });
        }
    });

    game.run();
}
