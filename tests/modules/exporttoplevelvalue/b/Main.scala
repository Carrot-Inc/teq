import top.*

// Explicit arguments: a call that leaves out another module's default lifts its receiver in the
// build over the products alone (`classes`), a gap of its own; the forwarded getters are called
// within one build by tests/cases/export_toplevel_value.
@main def main(): Unit =
  val t: T = 5
  println((teacherDashboard, count(1), t))
  println(new Outer(dashUi).Students.count(3))
