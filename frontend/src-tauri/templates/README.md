# Meeting Summary Templates

This directory contains template definitions for meeting summary generation.

## Bundled templates

- `daily_standup.json`: daily engineering or product updates.
- `project_sync.json`: project milestones, risks, decisions, and actions.
- `retrospective.json`: start/stop/continue retrospective.
- `sales_marketing_client_call.json`: client goals, commercial topics, and next steps.
- `standard_meeting.json`: general summary, decisions, actions, and discussion highlights.
- `psychatric_session.json`: upstream clinical-note template retained for compatibility; review privacy and regulatory requirements before use.

## Template Structure

Each template JSON file follows this schema:

```json
{
  "name": "Template Name",
  "description": "Brief description of the template's purpose",
  "sections": [
    {
      "title": "Section Title",
      "instruction": "Instructions for the LLM on what to extract/include",
      "format": "paragraph|list|string",
      "item_format": "Optional: Markdown table format for list items"
    }
  ]
}
```

## Custom Templates

Users can add custom templates to the application data directory:

- **macOS**: `~/Library/Application Support/Mingtily/templates/`
- **Windows**: `%APPDATA%\Mingtily\templates\`
- **Linux**: `$XDG_DATA_HOME/Mingtily/templates/` (normally `~/.local/share/Mingtily/templates/`)

Custom templates override built-in templates with the same filename.

## Template Fields

### Root Level
- `name` (required): Display name for the template
- `description` (required): Brief explanation of the template's use case
- `sections` (required): Array of section definitions

### Section Object
- `title` (required): Section heading text
- `instruction` (required): LLM guidance for this section
- `format` (required): One of `"paragraph"`, `"list"`, or `"string"`
- `item_format` (optional): Markdown formatting hint for list items (e.g., table structure)
- `example_item_format` (optional): Alternative formatting hint

## Usage in Code

Templates are loaded using the `templates` module:

```rust
use crate::summary::templates;

// Get a specific template
let template = templates::get_template("daily_standup")?;

// List available templates
let available = templates::list_templates();

// Validate custom template JSON
let custom_json = std::fs::read_to_string("custom.json")?;
let validated = templates::validate_template(&custom_json)?;
```
