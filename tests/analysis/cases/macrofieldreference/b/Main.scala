package macrofieldreferenceuse
case class Row(value: Long) derives macrofieldreference.Read
@main def run(): Unit =
  println(summon[macrofieldreference.Read[Row]].value(Row(3L)))
