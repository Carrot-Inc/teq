// A value class's `equals` and `hashCode` are the rule's where the only member of the name a trait holds goes by
// another target name, the traits and the classes in one module and their user in another: built whole and over
// the first module's products, the lines scalac 3.8.4 prints.
package renamed

import scala.annotation.targetName

trait Loose extends Any { @targetName("equalsRef") def equals(o: AnyRef): Boolean = false }
trait Mid extends Any with Loose
trait Seeded extends Any { @targetName("hashSeeded") def hashCode(seed: Int): Int = seed }

class Below(val x: Int) extends AnyVal with Loose
class Deep(val x: Int) extends AnyVal with Mid
class Hashed(val x: Int) extends AnyVal with Seeded
