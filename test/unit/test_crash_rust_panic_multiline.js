function run_test() {
  // Try crashing with a Rust panic
  do_crash(
    function() {
      Cc["@mozilla.org/xpcom/debug;1"]
        .getService(Ci.nsIDebug2)
        .rustPanic("OH NO\nOH NOES!");
    },
    function(mdump, extra) {
      Assert.equal(extra.MozCrashReason, "OH NO\nOH NOES!");
    },
    // process will exit with a zero exit status
    true
  );
}
