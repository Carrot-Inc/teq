// expect: 11:7: error: parameterized trait Required is indirectly implemented, needs to be implemented directly so that arguments can be passed
// expect: 12:8: error: parameterized trait Required is indirectly implemented, needs to be implemented directly so that arguments can be passed
// expect: 2 errors found
trait Empty()
trait NoArguments extends Empty
class Accepted extends NoArguments
trait Required(val value: Int)
trait Middle extends Required
class Direct extends Middle with Required(1)
class Inherited extends Direct
class Missing extends Middle
object MissingObject extends Middle
