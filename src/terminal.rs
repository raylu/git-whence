use unsegen::base::{Color, StyleModifier, Terminal};
use unsegen::container::{Container, ContainerManager, ContainerProvider, HSplit, Leaf};
use unsegen::input::{Input, Key, ScrollBehavior};
use unsegen::widget::builtin::LogViewer;
use unsegen::widget::{RenderingHints, Widget};

pub struct Pager {
	pub buffer: LogViewer,
}

impl Pager {
	pub fn new() -> Self {
		Pager {
			buffer: LogViewer::new(),
		}
	}
}

impl Container<()> for Pager {
	fn input(&mut self, input: Input, _: &mut ()) -> Option<Input> {
		input
			.chain(
				ScrollBehavior::new(&mut self.buffer)
					.backwards_on(Key::Char('k'))
					.forwards_on(Key::Char('j')),
			)
			.finish()
	}
	fn as_widget<'a>(&'a self) -> Box<dyn Widget + 'a> {
		Box::new(self.buffer.as_widget())
	}
}

#[derive(Clone, PartialEq, Debug)]
pub enum Index {
	Left,
	Right,
}

pub struct App {
	pub left: Pager,
	pub right: Option<Pager>,
}

impl App {
	pub fn new() -> App {
		App {
			left: Pager::new(),
			right: None,
		}
	}

	pub fn draw(&mut self, manager: &ContainerManager<App>, term: &mut Terminal) {
		manager.draw(
			term.create_root_window(),
			self,
			StyleModifier::new().fg_color(Color::Yellow),
			RenderingHints::default(),
		);
		term.present();
	}

	pub fn one_pane<'a>() -> Box<HSplit<'a, App>> {
		Box::new(HSplit::new(vec![(Box::new(Leaf::new(Index::Left)), 1.0)]))
	}
	pub fn two_pane<'a>() -> Box<HSplit<'a, App>> {
		Box::new(HSplit::new(vec![
			(Box::new(Leaf::new(Index::Left)), 0.5),
			(Box::new(Leaf::new(Index::Right)), 0.5),
		]))
	}
}

impl ContainerProvider for App {
	type Context = ();
	type Index = Index;
	fn get<'a, 'b: 'a>(&'b self, index: &'a Self::Index) -> &'b dyn Container<Self::Context> {
		match index {
			Index::Left => &self.left,
			Index::Right => match &self.right {
				Some(r) => r,
				None => &self.left,
			},
		}
	}
	fn get_mut<'a, 'b: 'a>(&'b mut self, index: &'a Self::Index) -> &'b mut dyn Container<Self::Context> {
		match index {
			Index::Left => &mut self.left,
			Index::Right => match &mut self.right {
				Some(ref mut r) => r,
				None => &mut self.left,
			},
		}
	}
	const DEFAULT_CONTAINER: Self::Index = Index::Left;
}
