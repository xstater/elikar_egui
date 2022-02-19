use std::{pin::Pin, sync::Arc, task::{Context, Poll}, time::Instant};
use crossbeam::channel::{Receiver, bounded};
use egui::{CtxRef, FontData, FontDefinitions, Pos2, RawInput, Rect};
use futures::{Stream, StreamExt};
use parking_lot::RwLock;
use xecs::{system::System, world::World};
use elikar::{clipboard::Clipboard, common::Spawner, events::{Events, Update}, ime::IME};

mod events;

pub fn build<S : Spawner>(spawner : &mut S,events : Events) -> Prepared {
    let keydown_rx = events::keydown(spawner, events.clone());
    let keyup_rx = events::keyup(spawner, events.clone());
    let mousedown_rx = events::mouse_down(spawner, events.clone());
    let mouseup_rx = events::mouse_up(spawner, events.clone());
    let motion_rx = events::mouse_motion(spawner, events.clone());
    let wheel_rx = events::mouse_wheel(spawner, events.clone());
    let editing_rx = events::text_editing(spawner, events.clone());
    let input_rx = events::text_input(spawner, events.clone());
    // let resized_rx = events::window_resized(spawner, events.clone());

    let (prepared_tx,prepared_rx) = bounded(1);
    let (renderer_tx,renderer_rx) = bounded(1);

    let events_ = events.clone();
    spawner.spawn_local(async move {
        let mut frame_start = events_.on_enter_frame();
        let world = frame_start.world();
        let mut ctx_ref = CtxRef::default();

        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "simfang".to_owned(),
            FontData::from_static(
                include_bytes!("..\\fonts\\simfang.ttf")
                ));
        fonts.fonts_for_family.get_mut(&egui::FontFamily::Proportional).unwrap()
            .insert(0,"simfang".to_owned());
        fonts.fonts_for_family.get_mut(&egui::FontFamily::Monospace).unwrap()
            .push("simfang".to_owned());
        ctx_ref.set_fonts(fonts);

        let (w,h) = {
            let world = world.read();
            let surface_config = world.resource_read::<wgpu::SurfaceConfiguration>().unwrap();
            (surface_config.width,surface_config.height)
        };

        let mut raw_input = RawInput {
            screen_rect: 
                Some(Rect::from_two_pos(
                        Pos2 { x:0.0, y:0.0 },
                        Pos2 { x: w as _, y: h as _})),
            pixels_per_point: None,
            .. RawInput::default()
        };

        let start_time = Instant::now();
        loop {
            // wait for frame start
            if let Some(_) = frame_start.next().await{
                // hanle input
                while let Ok(event) = keydown_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = keyup_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = mousedown_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = mouseup_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = motion_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = wheel_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = input_rx.try_recv() {
                    raw_input.events.push(event)
                }
                while let Ok(event) = editing_rx.try_recv() {
                    raw_input.events.push(event)
                }

                raw_input.time = Some(start_time.elapsed().as_secs_f64());
                ctx_ref.begin_frame(raw_input.take());

                renderer_tx.send(ctx_ref.clone()).unwrap();
                prepared_tx.send(ctx_ref.clone()).unwrap();
            }
        }
    });
    
    // renderer
    let events_ = events.clone();
    spawner.spawn_local(async move {
        let world = events_.world();
        let renderer_rx = renderer_rx;
        let mut render_pass = {
            let world = world.read();
            // create a render pass
            let device = world.resource_read::<wgpu::Device>().unwrap();
            let surface_config = world.resource_read::<wgpu::SurfaceConfiguration>().unwrap();
            egui_wgpu_backend::RenderPass::new(&device,surface_config.format,1)
        };

        // wait for render stage
        let mut render = events_.on_render();
        while let Some(_) = render.next().await {

            let world = world.read();
            // Unwrap never fails:
            // Render stage is behind of update stage
            // This Task was ran in local 
            // so this task must be run after update stage
            // so there must be a exact one valid number
            let ctx = renderer_rx.recv().unwrap();
            let font_image = ctx.font_image();
            let (output,shapes) = ctx.end_frame();
            let mesh = ctx.tessellate(shapes);

            // hanle output
            // copy to clipboard
            if !output.copied_text.is_empty() {
                let mut clipboard = world.resource_write::<Clipboard>().unwrap();
                clipboard.set(&output.copied_text).unwrap();
            }
            // ime
            {
                let mut ime = world.resource_write::<IME>().unwrap();
                if ctx.wants_keyboard_input() {
                    ime.start();
                } else {
                    ime.stop();
                }
                if ime.is_active() {
                    if let Some(cursor) = output.text_cursor_pos {
                        ime.set_area(cursor.x as _,cursor.y as _, 100, 50);
                    }
                }
            }

            // render
            let surface = world.resource_read::<wgpu::Surface>().unwrap();
            let output = surface.get_current_texture().unwrap();
            let output_view = output.texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let device = world.resource_read::<wgpu::Device>().unwrap();
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
                label: Some("egui_renderer_encoder"),
            });

            let surface_config = world.resource_read::<wgpu::SurfaceConfiguration>().unwrap();
            
            let screen_desc = egui_wgpu_backend::ScreenDescriptor{
                physical_width: surface_config.width,
                physical_height: surface_config.height,
                scale_factor: 1.0,
            };
            let queue = world.resource_read::<wgpu::Queue>().unwrap();

            render_pass.update_texture(&device, &queue, &font_image);
            render_pass.update_user_textures(&device,&queue);
            render_pass.update_buffers(
                &device,
                &queue,
                &mesh,
                &screen_desc);

            render_pass.execute(
                &mut encoder,
                &output_view,
                &mesh,
                &screen_desc,
                Some(wgpu::Color::WHITE)
            ).unwrap();

            
            queue.submit([encoder.finish()]);

            output.present();
        }
    });

    let update = events.on_update();
    Prepared{
        rx : prepared_rx,
        inner: Box::pin(update),
    }
}

// Ready when update stage and handled all events
pub struct Prepared {
    rx : Receiver<CtxRef>,
    inner : Pin<Box<Update>>
}

impl Stream for Prepared {
    type Item = CtxRef;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(_) = self.inner.as_mut().poll_next(cx) {
            if let Ok(ctx_ref) = self.rx.try_recv() {
                return Poll::Ready(Some(ctx_ref));
            }
        }
        Poll::Pending
    }
}

impl System for Prepared {
    fn world(&self) -> Arc<RwLock<World>> {
        self.inner.as_ref().world()
    }
}
