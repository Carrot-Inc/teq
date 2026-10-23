// A build's case (teq.lock): a and core under a.conf, b under b.conf, a and b sharing shared/src, a alone depending on
// core. Core's rewrite has round two revisit a and core, never b: b's round-one lint on Shared.scala stands beside a's
// outcome there, so the run prints it as the next run does.
object Core { val xs: List[Int] = List.empty }
