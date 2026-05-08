# SearchableVec Implementation & Text Highlighting Analysis

## 1. SearchableVec Location & Implementation

### Source
- **Library:** `gpui-component` (external dependency, not in workspace)
- **Git Repository:** `https://github.com/longbridge/gpui-component.git`
- **Revision:** `19e95aaed1ff05a717ec487661bd981fa7aeb28b`

### How SearchableVec is Used
From [server_settings_subpage.rs](crates/frontend/src/pages/instance/server_settings_subpage.rs):
```rust
use gpui_component::{
    select::{Select, SelectState, SelectEvent, SearchableVec}
};

// Creation
let versions = SearchableVec::new(vec!["system", "java8", "java17", "java21", "java25"]);

// Use with SelectState
let mut select_state = SelectState::new(versions, None, window, cx).searchable(true);
```

**Note:** SearchableVec is a generic struct from an external library. It implements `SelectDelegate` trait internally, but the actual implementation is not visible in this workspace.

---

## 2. How Select Renders Items from SearchableVec

### SelectItem & SelectDelegate Traits
Both traits are from `gpui_component`, but their interface is clear from implementations:

```rust
// From NamedDropdownItem implementation
impl<T: Clone> SelectItem for NamedDropdownItem<T> {
    type Value = Self;
    
    fn title(&self) -> SharedString {
        self.name.clone()        // Returns plain text string
    }
    
    fn value(&self) -> &Self::Value {
        self
    }
}

// From InstanceEntry implementation  
impl SelectItem for InstanceEntry {
    type Value = Self;
    
    fn title(&self) -> SharedString {
        self.name.clone()        // Plain text only!
    }
    
    fn value(&self) -> &Self::Value {
        &self
    }
}
```

### SelectDelegate Interface
```rust
impl<T: Clone> SelectDelegate for NamedDropdown<T> {
    type Item = NamedDropdownItem<T>;
    
    fn items_count(&self, _section: usize) -> usize {
        self.items.len()
    }
    
    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.items.get(ix.row)   // Returns items to render
    }
    
    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        for (ix, item) in self.items.iter().enumerate() {
            if item.value() == value {
                return Some(IndexPath::default().row(ix));
            }
        }
        None
    }
    
    fn perform_search(
        &mut self,
        _query: &str,
        _window: &mut Window,
        _: &mut Context<SelectState<Self>>,
    ) -> Task<()> {
        Task::ready(())          // SearchableVec handles search internally
    }
}
```

### Key Limitation
**The `SelectItem.title()` method returns only a `SharedString` (plain text).** There's no mechanism in the standard SelectItem/SelectDelegate interface to return rich text with styling (TextRun arrays with background colors).

---

## 3. Existing Text Highlighting in Codebase

### Example 1: GameOutputItem (game_output/mod.rs)
This shows the **working highlighting implementation** used for game output text:

```rust
// Structure with highlight data
pub struct GameOutputItem {
    text: Arc<[Arc<str>]>,
    index: usize,
    backup_total_lines_while_skipped: usize,
    total_lines: usize,
    // Stores (line_index, character_range) for highlighting
    highlighted_text: Option<(usize, Range<usize>)>,
    skip: bool,
}

// Computing highlights during search
impl GameOutputRoot {
    fn on_search_input_event(&mut self, ...) {
        // ... search logic ...
        for item in &mut item_state.items {
            let mut highlighted_text = None;
            
            if !item_state.search_query.is_empty() {
                for (line_index, line) in item.text.iter().enumerate() {
                    if let Some(found) = line.find(item_state.search_query.as_str()) {
                        highlighted_text = Some((
                            line_index,
                            found..found + item_state.search_query.as_str().len()
                        ));
                        break; // First match
                    }
                }
            }
            item.highlighted_text = highlighted_text;
        }
    }
}

// Rendering with TextRun styling
impl GameOutputItem {
    pub fn compute_wrapped_text<'a>(...) {
        let mut handle_segment = |wrapped_line: SharedString, from, to| {
            let runs: &[TextRun] = if let Some((highlight_line, highlight_range)) = &self.highlighted_text
                && *highlight_line == original_line_index
                && highlight_range.start < to
                && highlight_range.end > from
            {
                let highlight_start = highlight_range.start.max(from);
                let highlight_end = highlight_range.end.min(to);

                // Split into: before_highlight, highlighted, after_highlight
                &[
                    TextRun {
                        len: highlight_start - from,
                        font: font.clone(),
                        color: text_style.color,
                        background_color: text_style.background_color,  // Normal style
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                    TextRun {
                        len: highlight_end - highlight_start,
                        font: font.clone(),
                        color: gpui::white(),                           // White text
                        background_color: Some(gpui::hsla(0.0, 0.0, 0.5, 1.0)), // 50% gray BG
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                    TextRun {
                        len: to - highlight_end,
                        font: font.clone(),
                        color: text_style.color,
                        background_color: text_style.background_color,  // Normal style
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                ]
            } else {
                // No highlighting for this segment
                &[TextRun { ... normal rendering ... }]
            };
        };
    }
}
```

### Example 2: TextFieldLine (readonly_text_field.rs)
Simpler version for single-line highlighting:

```rust
struct TextFieldLine {
    line: Arc<str>,
    index: usize,
    backup_total_lines_while_skipped: usize,
    total_lines: usize,
    highlighted_text: Option<Range<usize>>,  // Simple: just character range
    skip: bool,
}

impl TextFieldLine {
    pub fn compute_wrapped_text<'a>(...) {
        let mut handle_segment = |wrapped_line: SharedString, from, to| {
            let runs: &[TextRun] = if let Some(highlight_range) = &self.highlighted_text
                && highlight_range.start < to
                && highlight_range.end > from
            {
                let highlight_start = highlight_range.start.max(from);
                let highlight_end = highlight_range.end.min(to);

                &[
                    TextRun {
                        len: highlight_start - from,
                        font: font.clone(),
                        color: text_style.color,
                        background_color: text_style.background_color,
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                    TextRun {
                        len: highlight_end - highlight_start,
                        font: font.clone(),
                        color: gpui::white(),                           // Highlighted text: white
                        background_color: Some(gpui::hsla(0.0, 0.0, 0.5, 1.0)), // Dark gray background
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                    TextRun {
                        len: to - highlight_end,
                        font: font.clone(),
                        color: text_style.color,
                        background_color: text_style.background_color,
                        underline: text_style.underline,
                        strikethrough: text_style.strikethrough,
                    },
                ]
            } else {
                // No highlight
                &[TextRun { ... }]
            };
        };
    }
}
```

---

## 4. Working Search Implementation Without Highlighting

### SearchHelper (component/search_helper.rs)
This is a working search implementation used by InstanceDropdown:

```rust
pub struct SearchHelper<T> {
    last_search: SharedString,
    items: Arc<[T]>,
    lower_keys: Vec<SharedString>,         // Lowercase versions of keys
    searched_starts_with: Vec<usize>,      // Indices of items starting with query
    searched_contains: Vec<usize>,         // Indices of items containing query
    searched: bool,
}

impl<T> SearchHelper<T> {
    pub fn new(items: Arc<[T]>, key: impl Fn(&T) -> SharedString) -> Self {
        let lower_keys = items
            .iter()
            .map(key)
            .map(|key| {
                if key.chars().any(char::is_uppercase) {
                    key.to_lowercase().into()
                } else {
                    key.clone()
                }
            })
            .collect();
        Self {
            last_search: SharedString::default(),
            items,
            lower_keys,
            searched_starts_with: Vec::new(),
            searched_contains: Vec::new(),
            searched: false,
        }
    }

    pub fn search(&mut self, query: &str) {
        if query.is_empty() {
            self.searched = false;
            self.last_search = SharedString::default();
            return;
        }

        if self.searched && query == self.last_search.as_str() {
            return;  // Already searched
        }

        // Smart re-filtering: if query starts with previous query, filter existing results
        if self.searched && query.starts_with(self.last_search.as_str()) {
            self.searched_contains.retain(|i| self.lower_keys[*i].contains(query));
            self.searched_starts_with.retain(|i| {
                let key = &self.lower_keys[*i];
                if !key.starts_with(query) {
                    if key.contains(query)
                        && let Err(insert_at) = self.searched_contains.binary_search(i)
                    {
                        self.searched_contains.insert(insert_at, *i);
                    }
                    false
                } else {
                    true
                }
            });
        } else {
            // New search: find all matches
            self.searched_contains.clear();
            self.searched_starts_with.clear();
            for (index, key) in self.lower_keys.iter().enumerate() {
                if key.starts_with(query) {
                    self.searched_starts_with.push(index);
                } else if key.contains(query) {
                    self.searched_contains.push(index);
                }
            }
        }

        self.searched = true;
        self.last_search = SharedString::new(query);
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if self.searched {
            let item_idx = if index >= self.searched_starts_with.len() {
                self.searched_contains.get(index - self.searched_starts_with.len())?
            } else {
                self.searched_starts_with.get(index)?
            };
            self.items.get(*item_idx)
        } else {
            self.items.get(index)
        }
    }

    pub fn iter(&self) -> Option<impl Iterator<Item = &T>> {
        if self.searched {
            Some(
                self.searched_starts_with
                    .iter()
                    .chain(self.searched_contains.iter())
                    .map(|i| self.items.get(*i).unwrap()),
            )
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        if self.searched {
            self.searched_starts_with.len() + self.searched_contains.len()
        } else {
            self.items.len()
        }
    }
}
```

### InstanceDropdown Using SearchHelper
```rust
pub struct InstanceDropdown {
    instances: Arc<[InstanceEntry]>,
    search: SearchHelper<InstanceEntry>,
}

impl InstanceDropdown {
    pub fn create(...) -> Entity<SelectState<Self>> {
        cx.new(|cx| {
            let instance_list = Self {
                instances: instances.clone(),
                search: SearchHelper::new(instances, |item| item.name.clone()),
            };
            SelectState::new(instance_list, None, window, cx).searchable(true)
        })
    }
}

impl SelectDelegate for InstanceDropdown {
    type Item = InstanceEntry;

    fn items_count(&self, _section: usize) -> usize {
        self.search.len()  // Respects filtered results
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.search.get(ix.row)  // Returns filtered items
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _: &mut Context<SelectState<Self>>,
    ) -> Task<()> {
        self.search.search(query);  // Performs filtering
        Task::ready(())
    }
}
```

---

## 5. What Would Be Needed to Add Highlighting to SearchableVec Dropdown Items

### Challenge
The fundamental issue is that `SelectItem::title()` returns only `SharedString` (plain text). The Select component from gpui-component renders items via `title()` and has no mechanism for returning styled TextRun arrays.

### Solution Structure: Enhanced Searchable Dropdown

To add highlighting to dropdown items, you would need:

```rust
// 1. Store highlight positions alongside items
pub struct SearchableVecWithHighlights<T> {
    items: Arc<[T]>,
    // NEW: Highlight ranges for each item
    highlight_ranges: Vec<Option<Vec<Range<usize>>>>,  // Per-item match ranges
    search: SearchHelper<T>,
}

// 2. Alternative: Wrapper type containing both item and highlights
#[derive(Clone)]
pub struct HighlightedItem<T: Clone> {
    item: T,
    title: SharedString,
    // NEW: Character ranges to highlight in title
    highlight_ranges: Vec<Range<usize>>,
}

impl<T: Clone> SelectItem for HighlightedItem<T> {
    type Value = T;
    
    fn title(&self) -> SharedString {
        self.title.clone()  // Still plain text - but we have highlight info
    }
    
    fn value(&self) -> &Self::Value {
        &self.item
    }
}

// 3. Custom rendering would require:
// - Custom SelectDelegate implementation (not using SelectItem.title())
// - Access to gpui's rendering layer to create TextRun arrays
// - Modify item rendering in the dropdown widget itself
// - This likely requires a custom Select component or renderer

pub struct CustomSelectWithHighlights<T> {
    items: Vec<HighlightedItem<T>>,
    search: SearchHelper<T>,
    // Store query to compute highlights on render
    search_query: SharedString,
}

impl<T: SelectItem> SelectDelegate for CustomSelectWithHighlights<T> {
    type Item = HighlightedItem<T>;
    
    fn items_count(&self, _section: usize) -> usize {
        self.search.len()
    }
    
    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        // Return item with pre-computed highlight ranges
        self.items.get(ix.row)
    }
    
    fn perform_search(&mut self, query: &str, ...) -> Task<()> {
        self.search_query = SharedString::new(query);
        self.search.search(query);
        
        // NEW: Compute highlight ranges for visible items
        for item in &mut self.items {
            if let Some(ranges) = compute_highlights(&item.title, query) {
                item.highlight_ranges = ranges;
            }
        }
        
        Task::ready(())
    }
}

fn compute_highlights(text: &str, query: &str) -> Option<Vec<Range<usize>>> {
    if query.is_empty() {
        return None;
    }
    
    let mut ranges = Vec::new();
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    
    for (pos, _) in text_lower.match_indices(&query_lower) {
        ranges.push(pos..pos + query.len());
    }
    
    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}
```

### Rendering the Highlighted Items
This is the difficult part - gpui-component's Select doesn't expose custom item rendering. You would need to:

1. **Extend SelectItem with custom rendering:**
   ```rust
   // Pseudo-code - would require gpui-component changes
   pub trait SelectItemWithRender: SelectItem {
       fn render_with_highlights(&self, cx: &mut Context) -> impl IntoElement;
   }
   ```

2. **Or create a custom dropdown component** that:
   - Uses SearchHelper for filtering
   - Implements custom render logic with TextRun arrays
   - Similar to how GameOutputItem renders with highlights

3. **Or monkey-patch by creating visual wrapper:**
   - Keep using SelectItem/SelectDelegate
   - Overlay visual highlighting via CSS/styling (limited)
   - Not ideal for precise character-level highlighting

---

## 6. Recommended Approach

### For SearchableVec (External Library)
Since SearchableVec is from an external library, you have limited options:

1. **Request Feature:** Open a PR or issue with gpui-component asking for custom item rendering support
2. **Create a Fork:** Maintain a local modified version of gpui-component with highlighting support
3. **Wrap the Search:** Keep using SearchableVec but add highlighting in a separate layer

### For Custom Dropdowns (Your Code)
For dropdowns you control (like server/mod selection), implement a custom component:

```rust
pub struct HighlightedSelectDropdown<T> {
    items: Arc<[T]>,
    search: SearchHelper<T>,
    search_query: SharedString,
}

// Implement SelectDelegate + custom rendering
// Use TextRun arrays similar to GameOutputItem
// This gives you full control over highlighting
```

---

## Summary Table

| Component | Search Support | Highlighting | Filtering | Location |
|-----------|---|---|---|---|
| **SearchableVec** | ✅ Built-in | ❌ None | ✅ Automatic | External (gpui-component) |
| **InstanceDropdown** | ✅ Via SearchHelper | ❌ None | ✅ Via SearchHelper | component/instance_dropdown.rs |
| **NamedDropdown** | ❌ No | ❌ None | ❌ No | component/named_dropdown.rs |
| **GameOutputItem** | ✅ Custom | ✅ TextRun-based | ✅ Custom | game_output/mod.rs |
| **TextFieldLine** | ✅ Custom | ✅ TextRun-based | ✅ Custom | component/readonly_text_field.rs |

