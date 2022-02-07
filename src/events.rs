use crossbeam::channel::{Receiver, unbounded};
use egui::{Key, Modifiers};
use futures::StreamExt;
use xecs::system::System;
use elikar::{clipboard::Clipboard, common::Spawner, events::Events, keyboard::{Code, Keyboard}, mouse::events::button::Button};

fn key_map(code : Code) -> Option<Key> {
    Some(match code {
        Code::Left => Key::ArrowLeft,
        Code::Up => Key::ArrowUp,
        Code::Right => Key::ArrowRight,
        Code::Down => Key::ArrowDown,

        Code::Escape => Key::Escape,
        Code::Tab => Key::Tab,
        Code::Backspace => Key::Backspace,
        Code::Space => Key::Space,
        Code::Return => Key::Enter,

        Code::Insert => Key::Insert,
        Code::Home => Key::Home,
        Code::Delete => Key::Delete,
        Code::End => Key::End,
        Code::Pagedown => Key::PageDown,
        Code::Pageup => Key::PageUp,

        Code::Kp0 | Code::_0 => Key::Num0,
        Code::Kp1 | Code::_1 => Key::Num1,
        Code::Kp2 | Code::_2 => Key::Num2,
        Code::Kp3 | Code::_3 => Key::Num3,
        Code::Kp4 | Code::_4 => Key::Num4,
        Code::Kp5 | Code::_5 => Key::Num5,
        Code::Kp6 | Code::_6 => Key::Num6,
        Code::Kp7 | Code::_7 => Key::Num7,
        Code::Kp8 | Code::_8 => Key::Num8,
        Code::Kp9 | Code::_9 => Key::Num9,

        Code::A => Key::A,
        Code::B => Key::B,
        Code::C => Key::C,
        Code::D => Key::D,
        Code::E => Key::E,
        Code::F => Key::F,
        Code::G => Key::G,
        Code::H => Key::H,
        Code::I => Key::I,
        Code::J => Key::J,
        Code::K => Key::K,
        Code::L => Key::L,
        Code::M => Key::M,
        Code::N => Key::N,
        Code::O => Key::O,
        Code::P => Key::P,
        Code::Q => Key::Q,
        Code::R => Key::R,
        Code::S => Key::S,
        Code::T => Key::T,
        Code::U => Key::U,
        Code::V => Key::V,
        Code::W => Key::W,
        Code::X => Key::X,
        Code::Y => Key::Y,
        Code::Z => Key::Z,

        _ => return None
    })
}

pub fn keydown<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_key_down = events.on_key_down();
        let tx = tx;
        while let Some(key) = on_key_down.next().await {
            let kmod = key.mod_state;
            if let Some(key) = key_map(key.code) {
                let event = egui::Event::Key{
                    key,
                    pressed: true,
                    modifiers: Modifiers{
                        alt: kmod.alt(),
                        ctrl: kmod.ctrl(),
                        shift: kmod.shift(),
                        mac_cmd: kmod.gui(),
                        command: 
                            kmod.left_ctrl() || kmod.left_gui(),
                    },
                };
                tx.send(event).unwrap();
            }
        }
    });

    rx
}

pub fn keyup<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_key_up = events.on_key_up();
        let world = on_key_up.world();
        let tx = tx;
        while let Some(key) = on_key_up.next().await {
            let kmod = key.mod_state;
            if let Some(key) = key_map(key.code) {
                let event = egui::Event::Key{
                    key,
                    pressed: false,
                    modifiers: Modifiers{
                        alt: kmod.alt(),
                        ctrl: kmod.ctrl(),
                        shift: kmod.shift(),
                        mac_cmd: kmod.gui(),
                        command: 
                            kmod.left_ctrl() || kmod.left_gui(),
                    },
                };
                tx.send(event).unwrap();

                let event = if key == Key::C && kmod.ctrl() {
                    Some(egui::Event::Copy)
                } else if key == Key::X && kmod.ctrl() {
                    Some(egui::Event::Cut)
                } else if key == Key::V && kmod.ctrl() {
                    let world = world.read().unwrap();
                    let clipboard = world.resource_ref::<Clipboard>().unwrap();
                    let text = clipboard.get().unwrap();
                    Some(egui::Event::Text(text))
                } else {
                    None
                };
                if let Some(event) = event {
                    tx.send(event).unwrap()
                }
            }
        }
    });

    rx
}

pub fn mouse_down<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_mouse_down = events.on_mouse_down();
        let world = on_mouse_down.world();
        let tx = tx;
        while let Some(mouse) = on_mouse_down.next().await {
            let button = match mouse.button {
                Button::Left => Some(egui::PointerButton::Primary),
                Button::Middle => Some(egui::PointerButton::Middle),
                Button::Right => Some(egui::PointerButton::Secondary),
                _ => None
            };
            if let Some(button) = button {
                let kmod = {
                    let world = world.read().unwrap();
                    let keyboard = world.resource_ref::<Keyboard>().unwrap();
                    keyboard.mod_state()
                };
                let event = egui::Event::PointerButton {
                    pos: egui::Pos2 { 
                        x: mouse.position.0 as f32,
                        y: mouse.position.1 as f32 
                    },
                    button,
                    pressed: true,
                    modifiers: Modifiers {
                        alt: kmod.alt(),
                        ctrl: kmod.ctrl(),
                        shift: kmod.shift(),
                        mac_cmd: kmod.gui(),
                        command: 
                            kmod.left_ctrl() || kmod.left_gui(),
                    },
                };
                tx.send(event).unwrap();
            }
        }
    });

    rx
}

pub fn mouse_up<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_mouse_up = events.on_mouse_up();
        let world = on_mouse_up.world();
        let tx = tx;
        while let Some(mouse) = on_mouse_up.next().await {
            let button = match mouse.button {
                Button::Left => Some(egui::PointerButton::Primary),
                Button::Middle => Some(egui::PointerButton::Middle),
                Button::Right => Some(egui::PointerButton::Secondary),
                _ => None
            };
            if let Some(button) = button {
                let kmod = {
                    let world = world.read().unwrap();
                    let keyboard = world.resource_ref::<Keyboard>().unwrap();
                    keyboard.mod_state()
                };
                let event = egui::Event::PointerButton {
                    pos: egui::Pos2 { 
                        x: mouse.position.0 as f32,
                        y: mouse.position.1 as f32 
                    },
                    button,
                    pressed: false,
                    modifiers: Modifiers {
                        alt: kmod.alt(),
                        ctrl: kmod.ctrl(),
                        shift: kmod.shift(),
                        mac_cmd: kmod.gui(),
                        command: 
                            kmod.left_ctrl() || kmod.left_gui(),
                    },
                };
                tx.send(event).unwrap();
            }
        }
    });

    rx
}

pub fn mouse_motion<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_mouse_motion = events.on_mouse_motion();
        let tx = tx;
        while let Some(mouse) = on_mouse_motion.next().await {
            let event = egui::Event::PointerMoved(egui::Pos2{
                x: mouse.position.0 as f32,
                y: mouse.position.1 as f32,
            });
            tx.send(event).unwrap();
        }
    });

    rx
}

pub fn mouse_wheel<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut on_mouse_wheel = events.on_mouse_wheel();
        let world = on_mouse_wheel.world();
        let tx = tx;
        while let Some(wheel) = on_mouse_wheel.next().await {
            let delta = egui::vec2(
                wheel.scrolled.0 as f32 * 8.0,
                wheel.scrolled.1 as f32 * 8.0
            );
            let kmod = {
                let world = world.read().unwrap();
                let keyboard = world.resource_ref::<Keyboard>().unwrap();
                keyboard.mod_state()
            };
            let event = if kmod.ctrl() {
                egui::Event::Zoom((delta.y / 125.0).exp())
            } else {
                egui::Event::Scroll(delta)
            };
            tx.send(event).unwrap();
        }
    });

    rx
}

pub fn text_input<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut text_input = events.on_text_input();
        let tx = tx;
        while let Some(input) = text_input.next().await {
            let event = egui::Event::Text(input.text);
            tx.send(event).unwrap();
        }
    });

    rx
}

pub fn text_editing<S : Spawner>(spawner : &mut S,events : Events) -> Receiver<egui::Event> {
    let (tx,rx) = unbounded();

    spawner.spawn_local(async move {
        let mut edit = events.on_text_editing();
        let tx = tx;
        while let Some(edit) = edit.next().await {
            let event = egui::Event::CompositionUpdate(edit.text);
            tx.send(event).unwrap();
        }
    });

    rx
}
