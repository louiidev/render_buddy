use glam::Vec4;
use pollster::block_on;
use render_buddy::{
    texture::{Image, TextureSamplerType},
    textured_rect::TexturedRect,
    transform::Transform,
    RenderBuddy,
};
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

    let mut render_buddy = block_on(RenderBuddy::new(&window, (1280, 720)));
    let img = image::load_from_memory(include_bytes!("./assets/bitbuddy.png")).unwrap();
    let dimensions = (img.width(), img.height());
    let handle = render_buddy.add_texture(Image {
        data: img.into_bytes(),
        dimensions,
        sampler: TextureSamplerType::Nearest,
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let (output, view, mut command_encoder) = render_buddy.begin();
            render_buddy.push(TexturedRect::new(handle), Transform::IDENTITY);
            render_buddy.render(
                &view,
                &mut command_encoder,
                Some(Vec4::new(0.1, 0.1, 0.1, 1.)),
            );
            render_buddy.end_frame(command_encoder, output);
        }
        Event::RedrawEventsCleared => *control_flow = ControlFlow::Poll,
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested {} => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
                render_buddy.resize_surface((physical_size.width, physical_size.height));
            }
            _ => {}
        },
        _ => {}
    });
}
