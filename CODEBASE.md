# CODEBASE.md: anirel Semantic Digest

> **Notice**: AI-optimized semantic index. Do not write narrative prose. Keep token density high.

## 1. System Topology & Data Flow
```text
Entrypoint ──> CLI/Parser ──> Domain Logic ──> Infra/IO
```

## 2. Global Constraints & Architecture Patterns
- **Primary Language**: Rust 2021 edition
- **Architectural Paradigm**: Role-based (domain/, infra/, api/cli/, tui/)
- **Hard Constraints**: <400 lines/file, <60 lines/fn, zero production unwrap(), 0 warnings.
- **Target Distribution**: Linux x86_64 standalone binary

## 3. Module & Interface Skeleton

### `src/app/actions.rs` (Role: general, Lines: 107)
- **Responsibility**: Core general logic in src/app/actions.rs
- **Imports**: use crossterm :: event :: { KeyCode , KeyEvent , KeyModifiers } , use crate :: app :: state :: View 
- **Types & Enums**:
  ```rust
  pub enum Action
  ```
- **Public Functions & Signatures**:
  ```rust
  fn action_for (key : KeyEvent , search_mode : bool) -> Action
  fn next_view (view : View) -> View
  fn prev_view (view : View) -> View
  ```

### `src/app/fuzzy.rs` (Role: general, Lines: 51)
- **Responsibility**: Core general logic in src/app/fuzzy.rs
- **Public Functions & Signatures**:
  ```rust
  fn matches (title : & str , filter : & str) -> bool
  ```

### `src/app/mod.rs` (Role: general, Lines: 7)
- **Responsibility**: Core general logic in src/app/mod.rs

### `src/app/scanner.rs` (Role: general, Lines: 84)
- **Responsibility**: Core general logic in src/app/scanner.rs
- **Imports**: use std :: collections :: HashSet , use tokio :: sync :: mpsc :: UnboundedSender , use crate :: domain :: model :: Library , use crate :: engine :: Engine , use crate :: infra :: anilist :: { BATCH_SIZE , ResolvedMedia } 
- **Types & Enums**:
  ```rust
  pub enum ScanEvent
  ```
- **Public Functions & Signatures**:
  ```rust
  async fn run (engine : Engine , library : Library , tx : UnboundedSender < ScanEvent >)
  ```

### `src/app/state.rs` (Role: general, Lines: 375)
- **Responsibility**: Core general logic in src/app/state.rs
- **Imports**: use crate :: app :: fuzzy , use crate :: config :: Scope , use crate :: domain :: cluster :: { Cluster , RelationGraph , build_clusters } , use crate :: domain :: model :: { Anime , Library , LibraryEntry } , use crate :: domain :: relation :: RelationKind 
- **Types & Enums**:
  ```rust
  pub enum View
  pub enum Focus
  pub enum Row
  pub struct Loading
  pub struct InitialLoading
  pub struct ScanStatus
  pub struct StatusMessage
  pub enum MessageKind
  pub struct AppState
  ```
- **Public Functions & Signatures**:
  ```rust
  fn label (& self) -> & 'static str
  fn new (user : String , library : Library , scope : Scope) -> Self
  fn set_initial_loading (& mut self , user : String)
  fn update_initial_progress (& mut self , count : usize)
  fn complete_initial_loading (& mut self , library : Library)
  fn fail_initial_loading (& mut self , error : String)
  fn ingest (& mut self , anime : Anime , relations : Vec < (i64 , RelationKind) >)
  fn rebuild (& mut self)
  fn rows (& self) -> & [Row]
  fn set_view (& mut self , view : View)
  fn set_filter (& mut self , filter : String)
  fn current_entry (& self) -> Option < & LibraryEntry >
  fn detail_cluster (& self) -> Option < & Cluster >
  fn detail_member (& self) -> Option < & crate :: domain :: cluster :: ClusterMember >
  fn show_cluster (& mut self , root_id : i64)
  fn show_focus_cluster (& mut self , cluster : crate :: domain :: cluster :: Cluster)
  fn selection_cluster (& self) -> Option < & Cluster >
  fn cluster_for (& self , mal_id : i64) -> Option < & Cluster >
  fn focus_target (& self) -> Option < & Anime >
  fn current_anime (& self) -> Option < & Anime >
  fn total_gaps (& self) -> usize
  fn library_entry (& self , mal_id : i64) -> Option < & LibraryEntry >
  ```

### `src/app/update.rs` (Role: general, Lines: 172)
- **Responsibility**: Core general logic in src/app/update.rs
- **Imports**: use crate :: app :: actions :: Action , use crate :: app :: state :: { AppState , Focus , MessageKind , View } 
- **Types & Enums**:
  ```rust
  pub enum Effect
  ```
- **Public Functions & Signatures**:
  ```rust
  fn apply (state : & mut AppState , action : Action) -> Option < Effect >
  fn notify (state : & mut AppState , text : impl Into < String > , kind : MessageKind)
  ```

### `src/art.rs` (Role: general, Lines: 301)
- **Responsibility**: Core general logic in src/art.rs
- **Imports**: use std :: path :: PathBuf , use ratatui :: style :: Color , use crate :: error :: { AnirelError , Result } 
- **Types & Enums**:
  ```rust
  pub struct CoverArt
  pub enum GraphicsSupport
  ```
- **Public Functions & Signatures**:
  ```rust
  fn resample (& self , width : usize , height : usize) -> Vec < u8 >
  fn detect_graphics () -> GraphicsSupport
  fn supports_truecolor () -> bool
  fn color (r : u8 , g : u8 , b : u8) -> Color
  fn cover_path (url : & str) -> PathBuf
  async fn ensure_cached (url : & str) -> Option < PathBuf >
  fn decode (path : & std :: path :: Path) -> Result < CoverArt >
  fn base64 (input : & [u8]) -> String
  fn native_image (protocol : GraphicsSupport , x : u16 , y : u16 , jpeg : & [u8]) -> Option < String >
  ```

### `src/cli.rs` (Role: cli, Lines: 85)
- **Responsibility**: Core cli logic in src/cli.rs
- **Imports**: use std :: path :: PathBuf , use clap :: { Parser , Subcommand , ValueEnum } 
- **Types & Enums**:
  ```rust
  pub struct Cli
  pub enum Command
  pub enum ExportFormat
  ```

### `src/commands.rs` (Role: general, Lines: 346)
- **Responsibility**: Core general logic in src/commands.rs
- **Imports**: use std :: path :: Path , use crate :: cli :: { Cli , Command , ExportFormat } , use crate :: config :: Config , use crate :: domain :: cluster :: { RelationGraph , build_clusters } , use crate :: engine :: Engine , use crate :: error :: { AnirelError , Result } , use crate :: output :: { self , FranchiseReport } 
- **Public Functions & Signatures**:
  ```rust
  async fn dispatch (cli : & Cli , config : & Config) -> Result < i32 >
  ```

### `src/config.rs` (Role: general, Lines: 90)
- **Responsibility**: Core general logic in src/config.rs
- **Imports**: use std :: env , use std :: path :: PathBuf , use std :: time :: Duration 
- **Types & Enums**:
  ```rust
  pub struct Scope
  pub struct Config
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new (user : impl Into < String >) -> Self
  fn library_dir (& self) -> PathBuf
  fn media_dir (& self) -> PathBuf
  fn state_dir (& self) -> PathBuf
  fn cache_dir () -> PathBuf
  fn remember_user (user : & str)
  fn last_user () -> Option < String >
  ```

### `src/domain/cluster.rs` (Role: domain, Lines: 481)
- **Responsibility**: Core domain logic in src/domain/cluster.rs
- **Imports**: use std :: collections :: { HashMap , HashSet } , use super :: model :: { Anime , Library , LibraryEntry } , use super :: relation :: RelationKind , use super :: watch_order :: order_members 
- **Types & Enums**:
  ```rust
  pub struct Edge
  pub struct RelationGraph
  pub struct ClusterMember
  pub struct Cluster
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new () -> Self
  fn insert_media (& mut self , anime : Anime)
  fn add_edge (& mut self , from : i64 , to : i64 , kind : RelationKind)
  fn media (& self , mal_id : i64) -> Option < & Anime >
  fn edges (& self) -> impl Iterator < Item = (i64 , & [Edge]) >
  fn edges_of (& self , mal_id : i64) -> & [Edge]
  fn contains (& self , mal_id : i64) -> bool
  fn media_count (& self) -> usize
  fn all_media (& self) -> Vec < Anime >
  fn neighbours (& self , mal_id : i64) -> Vec < i64 >
  fn is_missing (& self) -> bool
  fn is_unwatched (& self) -> bool
  fn needs_action (& self) -> bool
  fn total_duration_minutes (& self) -> Option < i64 >
  fn watched_duration_minutes (& self) -> i64
  fn left_duration_minutes (& self) -> i64
  fn owned_count (& self) -> usize
  fn missing_count (& self) -> usize
  fn action_count (& self) -> usize
  fn total_count (& self) -> usize
  fn missing (& self) -> impl Iterator < Item = & ClusterMember >
  fn member_ids (& self) -> Vec < i64 >
  fn total_duration_minutes (& self) -> i64
  fn watched_duration_minutes (& self) -> i64
  fn left_duration_minutes (& self) -> i64
  fn build_clusters (graph : & RelationGraph , library : & Library) -> Vec < Cluster >
  fn related_to (graph : & RelationGraph , library : & Library , seed : i64 , hops : usize ,) -> Option < Cluster >
  ```

### `src/domain/mod.rs` (Role: domain, Lines: 10)
- **Responsibility**: Core domain logic in src/domain/mod.rs
- **Imports**: pub use cluster :: { Cluster , ClusterMember , RelationGraph , build_clusters } , pub use model :: { Anime , Library , LibraryEntry , LibraryStatus } , pub use relation :: RelationKind 

### `src/domain/model.rs` (Role: domain, Lines: 247)
- **Responsibility**: Core domain logic in src/domain/model.rs
- **Imports**: use std :: collections :: HashMap , use serde :: { Deserialize , Serialize } 
- **Types & Enums**:
  ```rust
  pub struct Anime
  pub enum LibraryStatus
  pub struct LibraryEntry
  pub struct Library
  ```
- **Public Functions & Signatures**:
  ```rust
  fn display_title (& self) -> & str
  fn romaji_title (& self) -> & str
  fn english_title (& self) -> Option < & str >
  fn japanese_title (& self) -> Option < & str >
  fn url (& self) -> String
  fn is_aired (& self) -> bool
  fn format_rank (& self) -> u8
  fn episode_duration_minutes (& self) -> i64
  fn total_duration_minutes (& self) -> Option < i64 >
  fn format_total_duration (& self) -> Option < String >
  fn format_duration (minutes : i64) -> String
  fn from_code (code : i64) -> Self
  fn label (& self) -> & 'static str
  fn glyph (& self) -> & 'static str
  fn is_completed (& self) -> bool
  fn url (& self) -> String
  fn new (entries : Vec < LibraryEntry >) -> Self
  fn get (& self , mal_id : i64) -> Option < & LibraryEntry >
  fn get_mut (& mut self , mal_id : i64) -> Option < & mut LibraryEntry >
  fn contains (& self , mal_id : i64) -> bool
  fn len (& self) -> usize
  fn is_empty (& self) -> bool
  fn best_score_for (& self , mal_ids : & [i64]) -> Option < i64 >
  ```

### `src/domain/relation.rs` (Role: domain, Lines: 76)
- **Responsibility**: Core domain logic in src/domain/relation.rs
- **Imports**: use serde :: { Deserialize , Serialize } 
- **Types & Enums**:
  ```rust
  pub enum RelationKind
  ```
- **Public Functions & Signatures**:
  ```rust
  fn parse (raw : & str) -> Self
  fn label (& self) -> & 'static str
  fn is_franchise (& self) -> bool
  fn is_gap_relevant (& self) -> bool
  ```

### `src/domain/watch_order.rs` (Role: domain, Lines: 33)
- **Responsibility**: Core domain logic in src/domain/watch_order.rs
- **Imports**: use super :: cluster :: RelationGraph , use super :: model :: Anime , use super :: relation :: RelationKind 
- **Public Functions & Signatures**:
  ```rust
  fn order_members (graph : & RelationGraph , ids : & [i64]) -> Vec < (Anime , RelationKind) >
  ```

### `src/engine.rs` (Role: general, Lines: 207)
- **Responsibility**: Core general logic in src/engine.rs
- **Imports**: use std :: collections :: HashSet , use std :: path :: PathBuf , use std :: time :: Duration , use crate :: domain :: cluster :: RelationGraph , use crate :: domain :: model :: { Anime , Library } , use crate :: domain :: relation :: RelationKind , use crate :: error :: { AnirelError , Result } , use crate :: infra :: anilist :: { AniListClient , BATCH_SIZE , Media , ResolvedMedia } , use crate :: infra :: cache :: Cache , use crate :: infra :: mal_web :: MalWebClient 
- **Types & Enums**:
  ```rust
  pub struct Engine
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new (cache_dir : PathBuf , media_ttl : Duration , library_ttl : Duration) -> Self
  async fn library (& self , user : & str , force_refresh : bool) -> Result < Library >
  async fn library_with_progress < F > (& self , user : & str , force_refresh : bool , on_progress : F ,) -> Result < Library > where F : FnMut (usize) ,
  async fn resolve_batch (& self , ids : & [i64]) -> Result < Vec < Option < ResolvedMedia > > >
  async fn resolve_into (& self , ids : & [i64] , graph : & mut RelationGraph ,) -> Result < Vec < ResolvedMedia > >
  async fn expand (& self , graph : & mut RelationGraph , seeds : & [i64] , hops : usize ,) -> Result < () >
  fn materialise (media : & Media) -> Option < ResolvedMedia >
  async fn build_engine (config : & crate :: config :: Config) -> Result < Engine >
  async fn search_titles (query : & str) -> Result < Vec < Anime > >
  ```

### `src/error.rs` (Role: general, Lines: 41)
- **Responsibility**: Core general logic in src/error.rs
- **Imports**: use thiserror :: Error 
- **Types & Enums**:
  ```rust
  pub enum AnirelError
  ```
- **Public Functions & Signatures**:
  ```rust
  fn parse (context : impl Into < String > , detail : impl Into < String >) -> Self
  ```

### `src/infra/anilist.rs` (Role: infra, Lines: 352)
- **Responsibility**: Core infra logic in src/infra/anilist.rs
- **Imports**: use std :: time :: Duration , use serde :: { Deserialize , Serialize } , use serde_json :: { Value , json } , use crate :: domain :: model :: Anime , use crate :: domain :: relation :: RelationKind , use crate :: error :: { AnirelError , Result } 
- **Types & Enums**:
  ```rust
  pub struct Title
  pub struct CoverImage
  pub struct FuzzyDate
  pub struct RelationEdge
  pub struct Relations
  pub struct Media
  pub struct ResolvedMedia
  pub struct AniListClient
  ```
- **Public Functions & Signatures**:
  ```rust
  fn to_anime (& self) -> Option < Anime >
  fn new () -> Self
  async fn resolve (& self , mal_ids : & [i64]) -> Result < Vec < Option < Media > > >
  async fn search (_client : & AniListClient , query : & str) -> Result < Vec < Anime > >
  ```

### `src/infra/cache.rs` (Role: infra, Lines: 125)
- **Responsibility**: Core infra logic in src/infra/cache.rs
- **Imports**: use std :: path :: { Path , PathBuf } , use std :: time :: { Duration , SystemTime , UNIX_EPOCH } , use serde :: { Deserialize , Serialize , de :: DeserializeOwned } , use crate :: domain :: model :: { Library , LibraryEntry } , use crate :: error :: { AnirelError , Result } , use crate :: infra :: anilist :: Media 
- **Types & Enums**:
  ```rust
  pub struct Cache
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new (root : PathBuf) -> Self
  fn read_library (& self , user : & str , ttl : Duration) -> Option < Library >
  fn write_library (& self , user : & str , library : & Library) -> Result < () >
  fn read_media (& self , mal_id : i64 , ttl : Duration) -> Option < Media >
  fn write_media (& self , mal_id : i64 , media : & Media) -> Result < () >
  fn clear_media (& self) -> Result < () >
  ```

### `src/infra/mal_web.rs` (Role: infra, Lines: 428)
- **Responsibility**: Core infra logic in src/infra/mal_web.rs
- **Imports**: use serde :: Deserialize , use serde_json :: Value , use crate :: domain :: model :: { Anime , Library , LibraryEntry , LibraryStatus } , use crate :: domain :: relation :: RelationKind , use crate :: error :: { AnirelError , Result } 
- **Types & Enums**:
  ```rust
  pub struct MalWebClient
  ```
- **Public Functions & Signatures**:
  ```rust
  fn new () -> Self
  async fn fetch_library (& self , user : & str) -> Result < Library >
  async fn fetch_library_with_progress < F > (& self , user : & str , mut on_progress : F ,) -> Result < Library > where F : FnMut (usize) ,
  async fn fetch_related (& self , mal_id : i64) -> Result < Vec < (i64 , RelationKind) > >
  fn html_unescape (input : & str) -> String
  fn parse_related_entries (html : & str) -> Vec < (i64 , RelationKind) >
  ```

### `src/infra/mod.rs` (Role: infra, Lines: 6)
- **Responsibility**: Core infra logic in src/infra/mod.rs

### `src/infra/update.rs` (Role: infra, Lines: 337)
- **Responsibility**: Core infra logic in src/infra/update.rs
- **Imports**: use std :: fmt :: Write as _ , use std :: io :: Read as _ , use std :: path :: { Path , PathBuf } , use reqwest :: header :: { ACCEPT , USER_AGENT } , use serde :: Deserialize , use sha2 :: { Digest as _ , Sha256 } , use crate :: error :: { AnirelError , Result } 
- **Types & Enums**:
  ```rust
  pub enum Decision
  pub struct UpdateReport
  ```
- **Public Functions & Signatures**:
  ```rust
  fn decide (current : & str , latest : & str) -> Decision
  async fn run (force : bool) -> Result < UpdateReport >
  async fn run_with_progress < F > (force : bool , mut on_status : F) -> Result < UpdateReport > where F : FnMut (& str) ,
  ```

### `src/lib.rs` (Role: general, Lines: 17)
- **Responsibility**: Core general logic in src/lib.rs
- **Imports**: pub use error :: { AnirelError , Result } 

### `src/main.rs` (Role: general, Lines: 58)
- **Responsibility**: Core general logic in src/main.rs
- **Imports**: use anirel :: cli :: Cli , use anirel :: config :: { self , Config } , use anirel :: sys , use clap :: Parser 

### `src/output.rs` (Role: general, Lines: 393)
- **Responsibility**: Core general logic in src/output.rs
- **Imports**: use serde_json :: { Value , json } , use unicode_width :: UnicodeWidthStr , use crate :: domain :: cluster :: Cluster , use crate :: domain :: model :: Library 
- **Types & Enums**:
  ```rust
  pub struct MemberInfo
  pub struct FranchiseReport
  ```
- **Public Functions & Signatures**:
  ```rust
  fn url (& self) -> String
  fn display_title (& self) -> & str
  fn owned (& self) -> usize
  fn total (& self) -> usize
  fn missing (& self) -> Vec < & MemberInfo >
  fn missing_with (& self , include_other : bool) -> Vec < & MemberInfo >
  fn report_from_cluster (cluster : & Cluster , library : & Library) -> FranchiseReport
  fn report_from_members (name : String , members : Vec < (crate :: domain :: model :: Anime , crate :: domain :: relation :: RelationKind ,) > , library : & Library ,) -> FranchiseReport
  fn format_franchise (report : & FranchiseReport) -> String
  fn format_gaps (reports : & [FranchiseReport] , user : & str) -> String
  fn json_franchise (report : & FranchiseReport , user : & str) -> Value
  fn json_gaps (reports : & [FranchiseReport] , user : & str) -> Value
  fn markdown_report (reports : & [FranchiseReport] , user : & str) -> String
  fn csv_report (reports : & [FranchiseReport]) -> String
  ```

### `src/sys.rs` (Role: general, Lines: 95)
- **Responsibility**: Core general logic in src/sys.rs
- **Imports**: use std :: io :: { IsTerminal , Write } , use std :: process :: { Command , Stdio } 
- **Public Functions & Signatures**:
  ```rust
  fn open_in_browser (url : & str) -> Result < () , String >
  fn copy_to_clipboard (text : & str) -> Result < () , String >
  fn stdout_is_tty () -> bool
  fn color_enabled () -> bool
  ```

### `src/tui.rs` (Role: tui, Lines: 330)
- **Responsibility**: Core tui logic in src/tui.rs
- **Imports**: use std :: io :: Stdout , use std :: time :: Duration , use crossterm :: event :: { self , Event , KeyEventKind } , use ratatui :: Terminal , use ratatui :: backend :: CrosstermBackend , use tokio :: sync :: mpsc , use crate :: app :: actions :: action_for , use crate :: app :: scanner :: { self , ScanEvent } , use crate :: app :: state :: { AppState , Loading , MessageKind } , use crate :: app :: update :: { Effect , apply , notify } , use crate :: config :: Config , use crate :: domain :: model :: Library , use crate :: engine :: Engine , use crate :: output :: { self , FranchiseReport } , use crate :: sys 
- **Public Functions & Signatures**:
  ```rust
  async fn run (config : Config , engine : Engine , force_refresh : bool) -> anyhow :: Result < () >
  ```

### `src/ui/chrome.rs` (Role: tui, Lines: 158)
- **Responsibility**: Core tui logic in src/ui/chrome.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: Paragraph , use crate :: app :: state :: { AppState , View } , use crate :: ui :: theme :: Theme 
- **Public Functions & Signatures**:
  ```rust
  fn render_header (f : & mut Frame , area : Rect , state : & AppState)
  fn render_tabs (f : & mut Frame , area : Rect , state : & AppState)
  fn render_footer (f : & mut Frame , area : Rect , state : & AppState)
  ```

### `src/ui/help.rs` (Role: tui, Lines: 58)
- **Responsibility**: Core tui logic in src/ui/help.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: { Block , BorderType , Borders , Clear , Paragraph , Wrap } , use crate :: ui :: theme :: Theme 
- **Public Functions & Signatures**:
  ```rust
  fn render_help (f : & mut Frame , area : Rect)
  ```

### `src/ui/loading.rs` (Role: tui, Lines: 100)
- **Responsibility**: Core tui logic in src/ui/loading.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: { Block , BorderType , Borders , Clear , Paragraph , Wrap } , use crate :: app :: state :: { AppState , InitialLoading } , use crate :: ui :: theme :: Theme 
- **Public Functions & Signatures**:
  ```rust
  fn render_loading (f : & mut Frame , area : Rect , state : & AppState)
  ```

### `src/ui/mod.rs` (Role: tui, Lines: 90)
- **Responsibility**: Core tui logic in src/ui/mod.rs
- **Imports**: use ratatui :: Frame , use ratatui :: prelude :: * , use ratatui :: widgets :: { Paragraph , Wrap } , use crate :: app :: state :: AppState , use crate :: ui :: theme :: Theme 
- **Public Functions & Signatures**:
  ```rust
  fn draw (f : & mut Frame , state : & AppState)
  ```

### `src/ui/panes.rs` (Role: tui, Lines: 437)
- **Responsibility**: Core tui logic in src/ui/panes.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: { Gauge , List , ListItem , ListState , Paragraph , Wrap } , use unicode_width :: UnicodeWidthStr , use crate :: app :: state :: { AppState , Focus , Row , View } , use crate :: domain :: model :: LibraryStatus , use crate :: ui :: theme :: Theme , use crate :: ui :: widgets :: { fit , fit_title , panel } 
- **Public Functions & Signatures**:
  ```rust
  fn render_list (f : & mut Frame , area : Rect , state : & AppState)
  fn render_detail (f : & mut Frame , area : Rect , state : & AppState)
  ```

### `src/ui/poster.rs` (Role: tui, Lines: 244)
- **Responsibility**: Core tui logic in src/ui/poster.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: { Paragraph , Wrap } , use unicode_width :: UnicodeWidthStr , use crate :: app :: state :: { AppState , Focus } , use crate :: ui :: theme :: Theme , use crate :: ui :: widgets :: { fit , panel } 
- **Public Functions & Signatures**:
  ```rust
  fn render_poster (f : & mut Frame , area : Rect , state : & AppState)
  fn poster_width (terminal_width : u16) -> u16
  fn fits (terminal_width : u16) -> bool
  fn title_width (area : Rect) -> usize
  fn wrap_title (title : & str , width : usize , max_lines : usize) -> Vec < String >
  ```

### `src/ui/theme.rs` (Role: tui, Lines: 63)
- **Responsibility**: Core tui logic in src/ui/theme.rs
- **Imports**: use ratatui :: style :: { Color , Modifier , Style } 
- **Types & Enums**:
  ```rust
  pub struct Theme
  ```
- **Public Functions & Signatures**:
  ```rust
  fn base () -> Style
  fn muted () -> Style
  fn accent () -> Style
  fn ok () -> Style
  fn warn () -> Style
  fn missing () -> Style
  fn bold (style : Style) -> Style
  fn dim (style : Style) -> Style
  fn status_glyph (status : crate :: domain :: model :: LibraryStatus) -> & 'static str
  ```

### `src/ui/widgets.rs` (Role: tui, Lines: 56)
- **Responsibility**: Core tui logic in src/ui/widgets.rs
- **Imports**: use ratatui :: prelude :: * , use ratatui :: widgets :: { Block , BorderType , Borders } , use unicode_width :: { UnicodeWidthChar , UnicodeWidthStr } , use crate :: ui :: theme :: Theme 
- **Public Functions & Signatures**:
  ```rust
  fn panel (title : & str , focused : bool) -> Block < 'static >
  fn fit (text : & str , width : usize) -> String
  fn fit_title (title : & str , inner_width : u16 , meta_width : usize) -> String
  ```

## 4. Execution Lifecycle Trace
1. **Startup**: Entrypoint parses CLI flags & dispatches command.
2. **Execution**: Core domain logic processes inputs and evaluates rules.
3. **Persistence / I/O**: Domain logic calls infra for disk/terminal I/O.
4. **Exit**: Graceful termination with standard exit codes.

## 5. Verification Commands
```bash
cargo build --release --target x86_64-unknown-linux-gnu
cargo test --all-targets
cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
