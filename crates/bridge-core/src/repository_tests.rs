use super::*;
use crate::test_support::{key, Directory};

#[test]
fn repositories_worktrees_submodules_and_bare() -> Result<()> {
    let tmp = Directory::new();
    tmp.write("main/.git/HEAD", "ref: refs/heads/main\n");
    tmp.dir("main/child/deep");
    let main = resolve(tmp.0.join("main/child/deep").to_str().unwrap())?;
    assert_eq!(main.key, key(&tmp.0.join("main")));
    assert_eq!(main.worktree, main.key);
    assert_eq!(main.branch, "main");
    tmp.write(
        "main/.git/worktrees/wt/HEAD",
        "ref: refs/heads/feature/中文\n",
    );
    tmp.write("main/.git/worktrees/wt/commondir", "../..\n");
    tmp.write("relative/.git", "gitdir: ../main/.git/worktrees/wt\n");
    let relative = resolve(tmp.0.join("relative").to_str().unwrap())?;
    assert_eq!(relative.key, main.key);
    assert_eq!(relative.branch, "feature/中文");
    assert_eq!(relative.worktree, key(&tmp.0.join("relative")));
    tmp.write(
        "absolute/.git",
        &format!("gitdir: {}", tmp.0.join("main/.git/worktrees/wt").display()),
    );
    assert_eq!(
        resolve(tmp.0.join("absolute").to_str().unwrap())?.key,
        main.key
    );
    tmp.write("main/.git/modules/sub/HEAD", "0123456789abcdef\n");
    tmp.write("sub/.git", "gitdir: ../main/.git/modules/sub\n");
    let sub = resolve(tmp.0.join("sub").to_str().unwrap())?;
    assert_eq!(sub.key, key(&tmp.0.join("main/.git/modules/sub")));
    assert_eq!(sub.branch, "0123456");
    tmp.write("bare.git/HEAD", "ref: refs/heads/trunk\n");
    tmp.dir("bare.git/objects");
    tmp.dir("bare.git/refs");
    let bare = resolve(tmp.0.join("bare.git").to_str().unwrap())?;
    assert_eq!(bare.key, key(&tmp.0.join("bare.git")));
    assert_eq!(bare.branch, "trunk");
    Ok(())
}
#[test]
fn missing_common_and_bad_git_files_fall_back() -> Result<()> {
    let tmp = Directory::new();
    tmp.write("actual/HEAD", "123456789");
    tmp.write("wt/.git", "gitdir: ../actual");
    assert_eq!(
        resolve(tmp.0.join("wt").to_str().unwrap())?.key,
        key(&tmp.0.join("actual"))
    );
    for value in ["oops", "gitdir: ", "gitdir: ../missing"] {
        tmp.write("bad/.git", value);
        tmp.dir("bad/sub");
        let input = tmp.0.join("bad/sub");
        let project = resolve(input.to_str().unwrap())?;
        assert_eq!(project.key, key(&input));
        assert_eq!(project.branch, "-");
    }
    tmp.write("actual/commondir", "../missing");
    assert_eq!(
        resolve(tmp.0.join("wt").to_str().unwrap())?.key,
        key(&tmp.0.join("wt"))
    );
    Ok(())
}
