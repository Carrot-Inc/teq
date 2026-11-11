class Holder(val value: Int)

transparent inline def keep(inline input: Holder): Int = inline input match
  case bound: Holder if {
    val same: bound.value.type = bound.value
    true
  } =>
    val same: bound.value.type = bound.value
    def read(value: bound.value.type): Int = value
    read(same)
  case _ => -1

@main def run(): Unit =
  println(keep(new Holder(7)))
  println(keep(new Holder(11)))
