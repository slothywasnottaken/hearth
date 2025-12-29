use std::{
    io::{Write, stdout},
    path::{Path, PathBuf},
};

/// A toy-version of `git log`.
use clap::Parser;
use gix::{
    ObjectId, Remote,
    bstr::{BString, ByteSlice},
    date::time::format,
    revision::walk::Sorting,
};

fn main() {
    let args = Args::parse_from(gix::env::args_os());
    match run(args) {
        Ok(()) => {}
        Err(e) => eprintln!("error: {e}"),
    }
}

#[derive(Debug, clap::Parser)]
#[clap(name = "log", about = "git log example", version = option_env!("GIX_VERSION"))]
struct Args {
    /// Alternative git directory to use
    #[clap(name = "dir", long = "git-dir")]
    git_dir: Option<PathBuf>,
    /// Number of commits to return
    #[clap(short, long)]
    count: Option<usize>,
    /// Number of commits to skip
    #[clap(short, long)]
    skip: Option<usize>,
    /// Commits are sorted as they are mentioned in the commit graph.
    #[clap(short, long)]
    breadth_first: bool,
    /// Commits are sorted by their commit time in descending order.
    #[clap(short, long)]
    newest_first: bool,
    /// Show commits with the specified minimum number of parents
    #[clap(long)]
    min_parents: Option<usize>,
    /// Show commits with the specified maximum number of parents
    #[clap(long)]
    max_parents: Option<usize>,
    /// Show only merge commits (implies --min-parents=2)
    #[clap(long)]
    merges: bool,
    /// Show only non-merge commits (implies --max-parents=1)
    #[clap(long)]
    no_merges: bool,
    /// Reverse the commit sort order (and loads all of them into memory).
    #[clap(short, long)]
    reverse: bool,
    /// The ref-spec for the first commit to log, or HEAD.
    #[clap(name = "commit")]
    committish: Option<String>,
    /// The path interested in log history of
    #[clap(name = "path")]
    paths: Vec<PathBuf>,
}

fn run(args: Args) -> anyhow::Result<()> {
    let repo = gix::discover(args.git_dir.as_deref().unwrap_or(Path::new(".")))?;
    let id = repo.head_id().unwrap();
    let remote = repo
        .find_fetch_remote(Some("http://github.com/libgit2/libgit2/tree/main".into()))
        .unwrap();

    println!("{remote:?}");
    let commit = repo.head_commit().unwrap();
    let commit = commit.parent_ids();
    repo.commit("HEAD", "foo", id, commit).unwrap();

    Ok(())
}

struct LogEntryInfo {
    commit_id: String,
    parents: Vec<String>,
    author: BString,
    time: String,
    message: BString,
}
