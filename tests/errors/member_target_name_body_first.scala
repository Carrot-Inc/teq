import scala.annotation.targetName

abstract class Subject extends Parent:
  @targetName("pick") def choose(): Int = 1

trait Parent:
  def pick(): Int

// expect: 3:16: error: Name clash between defined and inherited member:
// expect+ def pick(): Int in trait Parent at line 7 and
// expect+ def choose(): Int in class Subject at line 4
// expect+ have the same name and type (): Int after erasure.
// expect: 1 error found
