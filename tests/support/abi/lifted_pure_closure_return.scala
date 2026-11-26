// A lambda with a `return` that stands as a pure statement is dropped with its body, and its return's key and handler
// stay: scalac writes no `$anonfun`, and its `run` allocates the key and catches `NonLocalReturnControl`. Every member
// of every class is listed (no `// abi:` line). The member set and the run are pinned (the run by
// tests/cases/lifted_pure_closure_return.scala), the body unpinned: the key and the handler are in `run`'s code, which
// the listing does not read.
object Main {
  def run(): Int = {
    (() => { return 3 })
    7
  }
  def main(args: Array[String]): Unit = println(run())
}
