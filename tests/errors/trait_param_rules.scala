// expect: 13:30: error: trait Labelled is already implemented by superclass Box, its constructor cannot be called again
// expect: 14:7: error: parameterized trait Labelled is indirectly implemented, needs to be implemented directly so that arguments can be passed
// expect: 16:23: error: trait Passing may not call constructor of trait Labelled
// expect: 17:23: error: missing argument for parameter label
// expect: 18:27: error: type mismatch: found String, required Int
// expect: 19:21: error: too many arguments: expected 2
// expect: 22:31: error: too many arguments for constructor Free in trait Free: (): Free
// expect: 7 errors found
trait Labelled(val label: String)
trait Shelved extends Labelled
trait Sized(width: Int, depth: Int = 1)
class Box extends Labelled("box")
class Again extends Box with Labelled("again")
class Indirect extends Shelved
class Direct extends Shelved with Labelled("direct")
trait Passing extends Labelled("no")
class Missing extends Labelled
class Wrong extends Sized("w")
class Extra extends Sized(1, 2, 3)
class Plain
trait Free
class Args extends Plain with Free(1)
