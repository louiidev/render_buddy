use glam::{Vec2, Vec3, Vec4};
use pollster::block_on;
use render_buddy::{camera::Camera, rect::Rect, RenderBuddy};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let window_builder = WindowBuilder::new().with_inner_size(LogicalSize::new(1280, 720));
    let event_loop = EventLoop::new();
    let window = window_builder.build(&event_loop).unwrap();

    let mut render_buddy = block_on(RenderBuddy::new(&window, (1280, 720))).unwrap();

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let mut render_ctx = render_buddy.begin();
            render_buddy.push(
                Rect::new(Vec2::new(100., 100.), Vec4::new(0.5, 0.5, 0.5, 1.)),
                Vec3::ZERO,
            );
            render_buddy.push(
                Rect::new(Vec2::new(100., 100.), Vec4::new(0.5, 0.5, 0.5, 0.75)),
                Vec3::new(25., 25., 0.),
            );
            render_buddy.push(
                Rect::new(Vec2::new(100., 100.), Vec4::new(0.5, 0.5, 0.5, 0.50)),
                Vec3::new(50., 50., 0.),
            );
            render_buddy.push(
                Rect::new(Vec2::new(100., 100.), Vec4::new(0.5, 0.5, 0.5, 0.25)),
                Vec3::new(75., 75., 0.),
            );
            render_buddy.render(
                &mut render_ctx,
                Some(Vec4::new(0.2, 0.2, 0.8, 1.)),
                &Camera::orthographic(),
            );
            render_buddy.end_frame(render_ctx);
        }
        Event::RedrawEventsCleared => *control_flow = ControlFlow::Poll,
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested {} => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                render_buddy.resize((physical_size.width, physical_size.height));
            }
            _ => {}
        },
        _ => {}
    });
}
