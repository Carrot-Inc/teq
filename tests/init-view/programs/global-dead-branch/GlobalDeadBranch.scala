object Mutable {
  var x = 0
}
object GlobalDeadBranch {
  val a = if (false) Mutable.x else 0
}
