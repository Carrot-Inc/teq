// expect: 11:60: error: ambiguous given instances for Int: gx, gy
// absent: was not considered because it was not imported
// A plain same-object tie of two givens is an ambiguity, whose message dotty gives no postscript
// (`MissingImplicitArgument.msgPostscript`): the note of a given not imported with `import given`
// (`ignoredInstanceNormalImport`) is a search's that found nothing alone. scalac 3.8.4: E172 "Ambiguous given instances:
// both given instance gx in object Main and given instance gy in object Main match type Int of parameter x of method
// summon in object Predef".
object Main:
  given gx: Int = 1
  given gy: Int = 2
  def main(args: Array[String]): Unit = println(summon[Int])
