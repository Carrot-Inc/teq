class OrderSafe {
  inline def touch(): Unit = { println(b); () }
  val b = 1
  touch()
}
