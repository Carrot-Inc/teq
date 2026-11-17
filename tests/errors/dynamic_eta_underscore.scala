import scala.language.dynamics
class Empty extends scala.Dynamic
object Uses:
  new Empty().missing _
// `e _` where `e` is a selection the rewrite cannot make: one error on the line, the missing dynamic method's
// (scalac's one error there is E099, whose text holds this one's); no second message that shows an erroneous type.
// expect: 4:3: error: value selectDynamic is not a member of Empty
// expect: 1 error found
// absent: 4:3: error: Only function types can be followed by _
