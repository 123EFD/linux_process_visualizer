use std::io::{Stdout, stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use linux_process_visualizer::app::App;

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

/// Enters raw mode and switches to the alternate screen buffer.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restores the terminal back to normal scrollback mode.
fn restore_terminal() -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        app.on_tick();
        terminal.draw(|f| linux_process_visualizer::ui::render(f, app))?;

        if event::poll(Duration::from_millis(1000))?
            && let Event::Key(key) = event::read()?
            && key.kind == crossterm::event::KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                KeyCode::Char('s') => app.cycle_sort(),
                KeyCode::Down | KeyCode::Char('j') => app.next_row(),
                KeyCode::Up | KeyCode::Char('k') => app.previous_row(),
                KeyCode::Home | KeyCode::Char('g') => app.selected_index = 0,
                KeyCode::End | KeyCode::Char('G') if !app.processes.is_empty() => {
                    app.selected_index = app.processes.len() - 1;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();

    let mut terminal = setup_terminal()?;
    let mut app = App::new();

    let result = run_app(&mut terminal, &mut app);

    // Clean up terminal unconditionally after event loop ends:
    let restore_result = restore_terminal();

    result.and(restore_result)
}
