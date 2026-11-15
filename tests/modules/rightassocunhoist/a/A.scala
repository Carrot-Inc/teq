// An inline method whose body conses a `summonInline` onto a call: the left operand, a call still to be inlined
// where the body is typed, is bound ahead of the right one (`Typer.typedInfixOp`) and stays bound when the
// expansion makes it the read of a given. The whole build took the binding back (`unhoist`: its initializer had
// become a stable read) and ran the right operand first; the build over this module's products kept it.
package probe
import scala.compiletime.summonInline
trait Item
given item: Item = { print("l"); new Item {} }
def right: List[Item] = { print("r"); Nil }
inline def result: List[Item] = summonInline[Item] :: right
