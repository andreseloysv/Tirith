#[test]
fn plan_add_rejects_invalid_checks() {
    let b = super::check_problem;
    assert!(["tirith run -s test", "tirith check", "cd collector && ls", "cargo test && tirith run"].iter().all(|c| b(c).is_some()) && ["cargo test -q x", "tirith plan show p", "cd collectors"].iter().all(|c| b(c).is_none()));
}
#[test]
fn a_rewritten_check_keeps_the_lane_on_its_item() {
    let (old, now) = ("Make the chat a PWA (check: `npm test`)", "Make the chat a PWA (check: `export PATH=/opt/homebrew/bin:$PATH; npm test`)");
    assert!(super::same(old, now) && super::same(old, "Make the chat a PWA") && !super::same(old, "Make the chat a PWA, with icons (check: `npm test`)"));
    // The lane started with the old text: merged, it ticks the item as the plan has it now, and no other.
    let (text, n) = super::tick_text(&format!("## 1. Step\n- [ ] {now}\n- [ ] Another\n"), &[("1".into(), old.into())]);
    assert_eq!((n, text.contains(&format!("- [x] {now}")), text.contains("- [ ] Another")), (1, true, true));
}
