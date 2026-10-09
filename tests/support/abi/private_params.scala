// A plain constructor parameter (`x: Int`, an anonymous `using Int`) that only its class's own
// methods read: scalac's private final field and no accessor (`Getters.noGetterNeeded`), the
// class of a subclass alike. A parameter an anonymous class reads keeps teq's accessor, where
// scalac widens the field under an expanded name (`private_params$Wide$$w`): not listed.
// abi: Named#x+final Named#x Anon#x$1+final Anon#x$1 Base#b+final Base#b get show twice
package private_params
class Named(x: Int):
  def get: Int = x + 1
class Anon(using Int):
  def get: Int = summon[Int] * 2
open class Base(b: Int):
  def show: Int = b
class Sub extends Base(3):
  def twice: Int = show * 2
