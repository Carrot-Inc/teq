// A guard indented after its `if`: the region it opens inside the case closes at the `=>`
// (`Scanners.postProcessToken`'s `ARROW` case, an `Indented` region whose outer one is `InCase`).
def f(x: Any): Int = x match
  case 1 if
      true
      && true => 1
  case _ => 0
@main def run(): Unit = println(f(1))
