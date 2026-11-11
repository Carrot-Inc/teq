enum Witness[A] {
  case Text extends Witness[String]
}
def method[A](proof: Witness[A]): A = proof match {
  case Witness.Text => "ok"
}
val literal: [A] => Witness[A] => A = [A] => (proof: Witness[A]) => proof match {
  case Witness.Text => "ok"
}

@main def polyfunctionGadtScope(): Unit = {
  println(method(Witness.Text))
  println(literal[String](Witness.Text))
}
