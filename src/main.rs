mod engine;
mod mesh;
mod pipeline;

use engine::VkEngine;

use winit::{
	event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
	event_loop::ControlFlow,
};
use winit::{event_loop::EventLoop, window::WindowBuilder};

fn main() {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Ecocide")
		.with_inner_size(winit::dpi::LogicalSize::new(800.0f64, 600.0f64))
		.build(&event_loop)
		.unwrap();
	let mut engine = VkEngine::new(&window);

	event_loop.run(move |event, _, control_flow| {
		*control_flow = ControlFlow::Poll;
		match event {
			Event::WindowEvent {
				event:
					WindowEvent::CloseRequested
					| WindowEvent::KeyboardInput {
						input:
							KeyboardInput {
								state: ElementState::Pressed,
								virtual_keycode: Some(VirtualKeyCode::Escape),
								..
							},
						..
					},
				..
			} => *control_flow = ControlFlow::Exit,
			Event::MainEventsCleared => engine.draw(),
			_ => (),
		}
	});
}
