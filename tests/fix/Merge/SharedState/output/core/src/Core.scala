// A build's case (teq.lock): a, b and core under one configuration, a and b sharing shared/src, a alone depending on
// core, and a project rule linting the shared file at its first visit, a's. Core's rewrite has round two revisit a:
// its shared file's results are a's own round one's, not b's later visit's, so the run prints the lint as the next does.
object Core { val xs: List[Int] = List.empty }
