class Order {
  inline def touch(): Unit = { println(b); () }
  touch()
  val b = 1
}
