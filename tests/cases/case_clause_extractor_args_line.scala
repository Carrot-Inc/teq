// An extractor's argument patterns on the line after its name, inside a case (dotty's `InCase`).
def f(x: Any): Int = x match
  case Some
      (1) => 1
  case _ => 0
@main def run(): Unit = println(f(Some(1)))
