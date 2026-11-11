class LiteralBox[T](val field: T) {
  def value: T = field
  def call(): T = field
}

def literalBox[T](value: T): LiteralBox[T] = new LiteralBox(value)

@main def parameterlessLiteralSelection(): Unit = {
  val selected: 1 = literalBox(1).value
  val invoked: 1 = literalBox(1).call()
  val field: 1 = literalBox(1).field
  println(selected)
  println(invoked)
  println(field)
}
