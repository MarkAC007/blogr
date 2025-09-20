use crate::content::{Post, PostManager, PostStatus};
use crate::project::Project;
use crate::utils::Console;
use anyhow::{anyhow, Result};

pub async fn handle_new(
    title: String,
    _template: String,
    draft: bool,
    slug: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    Console::info(&format!("Creating new post: '{}'", title));

    // Check if we're in a blogr project
    let project = Project::find_project()?
        .ok_or_else(|| anyhow!("Not in a blogr project. Run 'blogr init' first."))?;

    let config = project.load_config()?;

    // Parse tags
    let tags = tags
        .map(|t| t.split(',').map(|tag| tag.trim().to_string()).collect())
        .unwrap_or_default();

    // Set post status
    let status = if draft {
        PostStatus::Draft
    } else {
        PostStatus::Published
    };

    // Create new post
    let post = Post::new(
        title.clone(),
        config.blog.author.clone(),
        None, // Will use default description
        tags,
        slug,
        status,
    );

    // Save the post
    let post_manager = PostManager::new(project.posts_dir());
    let file_path = post_manager.save_post(&post)?;

    Console::success(&format!("Created new post: '{}'", title));
    println!("📝 Post saved to: {}", file_path.display());
    println!("🏷️  Slug: {}", post.metadata.slug);
    println!("📊 Status: {:?}", post.metadata.status);

    if !post.metadata.tags.is_empty() {
        println!("🏷️  Tags: {}", post.metadata.tags.join(", "));
    }

    println!();
    println!("💡 Next steps:");
    println!("  • Edit the post: blogr edit {}", post.metadata.slug);
    println!("  • Start dev server: blogr serve");
    if post.metadata.status == PostStatus::Draft {
        println!("  • Publish when ready: change status to 'published' in frontmatter");
    }

    Ok(())
}
