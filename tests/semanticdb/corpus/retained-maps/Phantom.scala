class Holder {
  type Phantom[A] = Int
  def phantom[A](a: A): Phantom[A] = 1
  val phantomString = phantom[String]("s")
}
