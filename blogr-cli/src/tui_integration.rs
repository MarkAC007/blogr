use crate::content::{Post, PostManager};
use crate::project::Project;
use crate::tui::theme::TuiTheme;
use crate::tui::{self, App, Event};
use anyhow::Result;

/// Launch the TUI editor for a post
pub async fn launch_editor(post: Post, project: &Project) -> Result<Post> {
    // Load theme configuration
    let config = project.load_config()?;

    // Create TUI theme from blog theme
    let tui_theme = if let Some(theme_config) = config.theme.config.get("primary_color") {
        let primary = theme_config.as_str().unwrap_or("#FF6B35");
        let secondary = config
            .theme
            .config
            .get("secondary_color")
            .and_then(|v| v.as_str())
            .unwrap_or("#F7931E");
        let background = config
            .theme
            .config
            .get("background_color")
            .and_then(|v| v.as_str())
            .unwrap_or("#2D1B0F");

        TuiTheme::from_blog_theme(primary, secondary, background)
    } else {
        TuiTheme::minimal_retro()
    };

    // Initialize TUI
    let mut tui = tui::init()?;
    tui.init()?;

    // Create post manager
    let post_manager = PostManager::new(project.posts_dir());

    // Create app
    let mut app = App::new(post, tui_theme, post_manager);

    // Main event loop
    let result = loop {
        // Draw the interface
        tui.draw(&mut app)?;

        // Handle events
        match tui.events.next()? {
            Event::Tick => {
                app.tick();
            }
            Event::Key(key_event) => {
                app.handle_key_event(key_event)?;
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }

        // Check if we should quit
        if !app.running {
            break Ok(app.post);
        }
    };

    // Cleanup
    tui.exit()?;

    result
}

/// Launch the configuration TUI
pub async fn launch_config_editor(project: &Project) -> Result<()> {
    // Load current configuration
    let config = project.load_config()?;

    println!();
    println!("🔧 Blogr Configuration");
    println!("{}", "=".repeat(50));
    println!();

    // Blog Information
    println!("📝 Blog Information:");
    println!("  Title: {}", config.blog.title);
    println!("  Author: {}", config.blog.author);
    println!("  Description: {}", config.blog.description);
    println!("  Base URL: {}", config.blog.base_url);
    if let Some(language) = &config.blog.language {
        println!("  Language: {}", language);
    }
    if let Some(timezone) = &config.blog.timezone {
        println!("  Timezone: {}", timezone);
    }
    println!();

    // Theme Configuration
    println!("🎨 Theme Configuration:");
    println!("  Current Theme: {}", config.theme.name);
    if !config.theme.config.is_empty() {
        println!("  Theme Settings:");
        for (key, value) in &config.theme.config {
            println!("    {}: {}", key, value);
        }
    } else {
        println!("  No custom theme settings configured");
    }
    println!();

    // GitHub Integration
    if let Some(github) = &config.github {
        println!("🐙 GitHub Integration:");
        println!("  Username: {}", github.username);
        println!("  Repository: {}", github.repository);
        println!("  Branch: {}", github.branch.as_deref().unwrap_or("main"));

        // Check for GitHub token in environment
        if std::env::var("GITHUB_TOKEN").is_ok() {
            println!("  Token: Configured via environment variable");
        } else {
            println!("  Token: Not configured (set GITHUB_TOKEN env var)");
        }
    } else {
        println!("🐙 GitHub Integration: Not configured");
    }
    println!();

    // Build Configuration
    println!("🔨 Build Configuration:");
    println!(
        "  Output Directory: {}",
        config.build.output_dir.as_deref().unwrap_or("dist")
    );
    println!("  Include Drafts: {}", config.build.drafts);
    println!("  Include Future Posts: {}", config.build.future_posts);
    println!();

    // Development Configuration
    println!("🚀 Development Configuration:");
    println!("  Port: {}", config.dev.port);
    println!("  Auto Reload: {}", config.dev.auto_reload);
    println!();

    // Configuration Instructions
    println!("⚙️ Configuration Management:");
    println!("{}", "─".repeat(50));
    println!("To modify your configuration, you can:");
    println!();
    println!("1. 📝 Edit blogr.toml directly:");
    println!(
        "   - Located at: {}",
        project.root.join("blogr.toml").display()
    );
    println!("   - Use any text editor to modify settings");
    println!();
    println!("2. 🎨 Change themes:");
    println!("   - blogr theme list         # View available themes");
    println!("   - blogr theme set <name>   # Switch to a different theme");
    println!("   - blogr theme info <name>  # View theme details");
    println!();
    println!("3. 🐙 Configure GitHub integration:");
    println!("   - Set GITHUB_TOKEN environment variable");
    println!("   - Update [github] section in blogr.toml");
    println!("   - Run 'blogr deploy' to deploy to GitHub Pages");
    println!();
    println!("4. ✅ Validate configuration:");
    println!("   - blogr project check      # Validate project structure");
    println!("   - blogr project info       # View project information");
    println!();
    println!("💡 Changes to blogr.toml take effect immediately.");
    println!("   Run 'blogr build' or 'blogr serve' to see updates.");

    Ok(())
}
