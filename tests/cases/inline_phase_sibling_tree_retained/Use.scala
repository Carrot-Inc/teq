//> using options -Yretain-trees
// teq: --retain-trees
class C:
  lazy val d1: String = M.inspect
  lazy val d2: String = M.inspect
  lazy val d3: String = M.inspect
@main def run(): Unit = println(new C().d1)
