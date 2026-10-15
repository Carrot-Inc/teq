// An export through a val whose type is an object's forwards the object's members whatever
// spells the type: an alias of `Target.type`, `Target.type` itself, or the type inferred from
// `Target`, as `Namer` types an export's qualifier ahead and forwards through its type.
object Target:
  def result: Int = 42
  def twice(n: Int): Int = n * 2

type Alias = Target.type
val alias: Alias = Target
val direct: Target.type = Target
val inferred = Target

object Facade:
  export alias.result

object Direct:
  export direct.twice

object Inferred:
  export inferred.{result as answer}

@main def main(): Unit =
  println(Facade.result)
  println(Direct.twice(4))
  println(Inferred.answer)
