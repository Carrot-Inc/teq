// A polymorphic function literal that begins a brace block gives the lines after its function literal's arrow to
// that literal as its body, as a function literal that begins the block has them: one literal, one nested in
// another's body, one of no parameters. scalac prints the same lines.
def one(f: [A] => A => String): String = f[Int](1)
def nested(f: [A] => A => [B] => B => String): String = f[Int](1)[String]("b")
def unit(f: [A] => () => Unit): Unit = f[Int]()

@main def main(): Unit =
  println(one { [A] => (x: A) =>
    val s = x.toString
    s + "!"
  })
  println(nested { [A] => (x: A) => [B] => (y: B) =>
    val s = x.toString + y.toString
    s + "!"
  })
  unit { [A] => () =>
    println(1)
    println(2)
  }
