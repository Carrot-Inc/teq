object Old {
  @deprecated("old", "1") def v: Int = 1
  @deprecated("older", "1") def w: Int = 2
}
object A { def x: Int = Old.v + Mac.twice(Old.w) }
object B { def y: Int = Old.v + Old.w }
