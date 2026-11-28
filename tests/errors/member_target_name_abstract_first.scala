import scala.annotation.targetName

trait Parent:
  def pick(): Int

abstract class Subject extends Parent:
  @targetName("pick") def choose(): Int = 1

// expect: 6:16: error: Name clash between defined and inherited member:
// expect+ def pick(): Int in trait Parent at line 4 and
// expect+ def choose(): Int in class Subject at line 7
// expect+ have the same name and type (): Int after erasure.
// expect: 1 error found
