// A top-level export through a top-level lazy val, and an export in a nested object through its
// enclosing class's val: the qualifier is typed as a path in the clause's scope (dotty's
// `Namer.exportForwarders`, `typedAheadExpr`), and the forwarders are members of the file's
// `<source>$package` (`Desugar.packageDef`) and of the object.
package top

class Dash:
  type T = Int
  val teacher = 7
  def count(n: Int = 2) = n + teacher

lazy val dashUi = new Dash
export dashUi.{teacher as teacherDashboard, count, T}

class Outer(val colls: Dash):
  object Students:
    export colls.count

@main def topMain() =
  val t: T = 5
  println((teacherDashboard, count(), count(1), t))
  println(new Outer(dashUi).Students.count())
  println(new Outer(new Dash).Students.count(10))
