package top

// A top-level export through a top-level lazy val, its forwarders members of the file's
// `<source>$package` (dotty's `Desugar.packageDef`), and an export in a nested object through
// its enclosing class's val, which another module calls through the products.
class Dash:
  type T = Int
  val teacher = 7
  def count(n: Int = 2) = n + teacher

lazy val dashUi = new Dash
export dashUi.{teacher as teacherDashboard, count, T}

class Outer(val colls: Dash):
  object Students:
    export colls.count
