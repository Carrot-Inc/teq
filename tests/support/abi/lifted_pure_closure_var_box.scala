// A lambda that stands as a pure statement is dropped with its body, and the var it reads stays boxed: scalac writes no
// `$anonfun`, and its `run` keeps `x` in a `scala.runtime.IntRef`. Every member of every class is listed (no `// abi:`
// line). The member set and the run are pinned (the run by tests/cases/lifted_pure_closure_var_box.scala), the body
// unpinned: the box is in `run`'s code, which the listing does not read.
object Main {
  def run(n: Int): Int = {
    var x = n
    (() => x)
    x += 1
    x
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
