// expect: 51:47: error: forward reference to Later (defined on line 53) extends over the definition of held (on line 52)
// expect: 56:44: error: forward reference to Later (defined on line 58) extends over the definition of held (on line 57)
// expect: 61:49: error: forward reference to Later (defined on line 63) extends over the definition of held (on line 62)
// expect: 66:47: error: forward reference to Marked (defined on line 68) extends over the definition of held (on line 67)
// expect: 71:47: error: forward reference to Later (defined on line 73) extends over the definition of held (on line 72)
// expect: 76:47: error: forward reference to Shade (defined on line 78) extends over the definition of held (on line 77)
// expect: 81:52: error: forward reference to Later (defined on line 84) extends over the definition of held (on line 82)
// expect: 87:48: error: forward reference to Later (defined on line 89) extends over the definition of held (on line 88)
// expect: 92:54: error: forward reference to Later (defined on line 94) extends over the definition of held (on line 93)
// expect: 97:44: error: forward reference to Later (defined on line 99) extends over the definition of held (on line 98)
// expect: 102:51: error: forward reference to Failure (defined on line 104) extends over the definition of held (on line 103)
// expect: 107:53: error: forward reference to Later (defined on line 109) extends over the definition of held (on line 108)
// expect: 112:47: error: forward reference to Later (defined on line 114) extends over the definition of held (on line 113)
// expect: 117:62: error: forward reference to Later (defined on line 119) extends over the definition of held (on line 118)
// expect: 122:47: error: forward reference to Later (defined on line 124) extends over the definition of held (on line 123)
// expect: 127:47: error: forward reference to Later (defined on line 129) extends over the definition of held (on line 128)
// expect: 132:54: error: forward reference to Later (defined on line 134) extends over the definition of held (on line 133)
// expect: 137:63: error: forward reference to Later (defined on line 139) extends over the definition of held (on line 138)
// expect: 142:38: error: forward reference to Later (defined on line 143) extends over the definition of held (on line 142)
// expect: 146:65: error: forward reference to Later (defined on line 148) extends over the definition of held (on line 147)
// expect: 152:34: error: forward reference to Later (defined on line 155) extends over the definition of held (on line 154)
// expect: 158:66: error: forward reference to Far (defined on line 161) extends over the definition of held (on line 159)
// expect: 164:62: error: forward reference to Last (defined on line 167) extends over the definition of held (on line 165)
// expect: 170:62: error: forward reference to Last (defined on line 173) extends over the definition of held (on line 171)
// expect: 177:44: error: forward reference to Short (defined on line 179) extends over the definition of held (on line 178)
// expect: 184:44: error: forward reference to Second (defined on line 186) extends over the definition of held (on line 185)
// expect: 189:47: error: forward reference to Later (defined on line 191) extends over the definition of held (on line 190)
// expect: 194:44: error: forward reference to Later (defined on line 196) extends over the definition of held (on line 195)
// expect: 199:44: error: forward reference to late (defined on line 201) extends over the definition of held (on line 200)
// expect: 204:44: error: forward reference to Later (defined on line 206) extends over the definition of held (on line 205)
// expect: 209:47: error: forward reference to Shadowed (defined on line 211) extends over the definition of held (on line 210)
// expect: 217:54: error: forward reference to Later (defined on line 216) extends over the definition of held (on line 215)
// expect: 220:47: error: forward reference to Later (defined on line 222) extends over the definition of held (on line 221)
// expect: 225:61: error: forward reference to Later (defined on line 227) extends over the definition of held (on line 226)
// expect: 231:15: error: forward reference to Later (defined on line 236) extends over the definition of held (on line 235)
// expect: 239:50: error: forward reference to Later (defined on line 241) extends over the definition of held (on line 240)
// expect: 244:47: error: forward reference to Later (defined on line 246) extends over the definition of held (on line 245)
// expect: 249:47: error: forward reference to Later (defined on line 251) extends over the definition of held (on line 250)
// expect: 254:44: error: forward reference to Later (defined on line 256) extends over the definition of held (on line 255)
// expect: 262:38: error: forward reference to Later (defined on line 263) extends over the definition of held (on line 262)
// expect: 267:35: error: forward reference to Later (defined on line 268) extends over the definition of held (on line 267)
// expect: 272:39: error: forward reference to Later (defined on line 273) extends over the definition of held (on line 272)
// expect: 277:75: error: forward reference to Later (defined on line 278) extends over the definition of held (on line 277)
// expect: 282:41: error: forward reference to Later (defined on line 283) extends over the definition of held (on line 282)
// expect: 44 errors found
// A typed pattern whose written type names a later class, trait, enum or type alias of its block, or by `T.type`
// a later object or lazy value, over a value's definition: scalac's check reads the pattern's wildcard, an
// identifier of the written type, told at the type for a typed variable and at the `_` for a typed wildcard.
object Patterns:
  def typedVariable(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def typedWildcard(n: Int): Int =
    def isIt(x: Any): Int = x match { case _: Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def boundWildcard(n: Int): Int =
    def isIt(x: Any): Int = x match { case l @ (_: Later) => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def laterTrait(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Marked => 1; case _ => 0 }
    val held: Int = isIt(n)
    trait Marked
    held
  def laterCaseClass(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Later => l.v; case _ => 0 }
    val held: Int = isIt(n)
    case class Later(v: Int)
    held
  def laterEnum(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Shade => 1; case _ => 0 }
    val held: Int = isIt(n)
    enum Shade { case Dark, Light }
    held
  def inComponent(n: Int): Int =
    def isIt(x: Any): Int = x match { case Some(l: Later) => 1; case Some(_: Other) => 2; case _ => 0 }
    val held: Int = isIt(n)
    class Other
    class Later
    held
  def inCons(n: Int): Int =
    def isIt(x: Any): Int = x match { case (l: Later) :: Nil => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def inBindInComponent(n: Int): Int =
    def isIt(x: Any): Int = x match { case Some(y @ (_: Later)) => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def inAlternative(n: Int): Int =
    def isIt(x: Any): Int = x match { case _: Later | _: String => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def inCatch(n: Int): Int =
    def isIt(x: Int): Int = try x catch { case e: Failure => 0 }
    val held: Int = isIt(n)
    class Failure extends RuntimeException
    held
  def inPartialFunction(n: Int): Int =
    def isIt: PartialFunction[Any, Int] = { case l: Later => 1 }
    val held: Int = isIt.applyOrElse(n, (_: Any) => 0)
    class Later
    held
  def inValuePattern(n: Int): Int =
    def first(p: (Any, Int)): Int = { val (l: Later, k) = p: @unchecked; k }
    val held: Int = n
    class Later
    held
  def inGenerator(n: Int): Int =
    def count(xs: List[Any]): Int = { var k = 0; for case l: Later <- xs do k += 1; k }
    val held: Int = count(List(n))
    class Later
    held
  def parenthesised(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: (Later) => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def annotated(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Later @unchecked => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def inInlineMethodOfTheBlock(n: Int): Int =
    inline def isIt(x: Any): Int = x match { case l: Later => 1; case _ => 0 }
    val held: Int = n
    class Later
    held
  def inClassOfTheBlock(n: Int): Int =
    class Checker { def isIt(x: Any): Int = x match { case l: Later => 1; case _ => 0 } }
    val held: Int = new Checker().isIt(n)
    class Later
    held
  def inItsOwnInitialiser(n: Int): Int =
    val held: Any => Int = { case l: Later => 1; case _ => 0 }
    class Later
    held(n)
  def inGuardsMatch(n: Int): Int =
    def isIt(x: Any): Int = x match { case _ if (x match { case _: Later => true; case _ => false }) => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def inInnerBlock(n: Int): Int =
    def isIt(x: Any): Int =
      val inner = x match { case _: Later => 1; case _ => 0 }
      inner
    val held: Int = isIt(n)
    class Later
    held
  def twoCasesTheFarther(n: Int): Int =
    def isIt(x: Any): Int = x match { case a: Near => 1; case b: Far => 2; case _ => 0 }
    val held: Int = isIt(n)
    class Near
    class Far
    held
  def twoInOnePattern(n: Int): Int =
    def isIt(x: (Any, Any)): Int = x match { case (_: Later, _: Last) => 1; case _ => 0 }
    val held: Int = isIt((n, n))
    class Later
    class Last
    held
  def besideConstructorPattern(n: Int): Int =
    def isIt(x: (Any, Any)): Int = x match { case (_: Later, Last(_)) => 1; case _ => 0 }
    val held: Int = isIt((n, n))
    class Later
    case class Last(i: Int)
    held
  def laterAlias(n: Int): Int =
    class Plain
    def isIt(x: Any): Int = x match { case _: Short => 1; case _ => 0 }
    val held: Int = isIt(n)
    type Short = Plain
    held
  def laterAliasOfAlias(n: Int): Int =
    class Plain
    type First = Plain
    def isIt(x: Any): Int = x match { case _: Second => 1; case _ => 0 }
    val held: Int = isIt(n)
    type Second = First
    held
  def singletonVariable(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Later.type => 1; case _ => 0 }
    val held: Int = isIt(n)
    object Later
    held
  def singletonWildcard(n: Int): Int =
    def isIt(x: Any): Int = x match { case _: Later.type => 1; case _ => 0 }
    val held: Int = isIt(n)
    object Later
    held
  def singletonOfLazyValue(n: Int): Int =
    def isIt(x: Any): Int = x match { case _: late.type => 1; case _ => 0 }
    val held: Int = isIt(n)
    lazy val late: String = "late"
    held
  def singletonOfCompanion(n: Int): Int =
    def isIt(x: Any): Int = x match { case _: Later.type => 1; case _ => 0 }
    val held: Int = isIt(n)
    case class Later(i: Int)
    held
  def outerClassOfTheName(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Shadowed => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Shadowed
    held
  def laterInlineMethod(n: Int): Int =
    def first(x: Any): Int = pick(x)
    val held: Int = first(n)
    class Later
    inline def pick(x: Any): Int = x match { case l: Later => 1; case _ => 0 }
    held
  def commentInParentheses(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: ( /* between */ Later) => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def commentBeforeTheType(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: /* between */ Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def lineCommentInParentheses(n: Int): Int =
    def isIt(x: Any): Int = x match {
      case l: ( // between
        Later) => 1
      case _ => 0
    }
    val held: Int = isIt(n)
    class Later
    held
  def givenPattern(n: Int): Int =
    def isIt(x: Any): Int = x match { case given Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    class Later
    held
  def singletonVariableInParentheses(n: Int): Int =
    def isIt(x: Any): Int = x match { case l: (Later.type) => 1; case _ => 0 }
    val held: Int = isIt(n)
    object Later
    held
  def aliasOfTypeParameter[A: scala.reflect.ClassTag](n: Int): Int =
    def isIt(x: Any): Int = x match { case l: Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    type Later = A
    held
  def aliasOfTypeParameterWildcard[A](n: Int)(using scala.reflect.TypeTest[Any, A]): Int =
    def isIt(x: Any): Int = x match { case _: Later => 1; case _ => 0 }
    val held: Int = isIt(n)
    type Later = A
    held
  // A `for`'s value definition stands as a pattern where its run of aliases is packed with the element (a guard
  // follows, an alias of the run holds a nested given pattern), and where it is no typed variable.
  def forAliasBeforeGuard(n: Int): Int =
    def make(y: Any): Later = null
    val held = for (y <- List(n); l: Later = make(y); if l == null) yield 1
    class Later
    held.sum
  def forAliasWildcard(n: Int): Int =
    def make(y: Any): Later = null
    val held = for (y <- List(n); _: Later = make(y)) yield 1
    class Later
    held.sum
  def forAliasInParentheses(n: Int): Int =
    def make(y: Any): Later = null
    val held = for (y <- List(n); (l: Later) = make(y)) yield 1
    class Later
    held.sum
  def forAliasAfterNestedGiven(n: Int): Int =
    def make(y: Any): Later = null
    val held = for (y <- List(n); (given Tagged, k) = (new Tagged, 1); l: Later = make(y)) yield k
    class Later
    held.sum
  def forGivenAlias(n: Int): Int =
    def make(y: Any): Later = null
    val held = for (y <- List(n); given Later = make(y)) yield 1
    class Later
    held.sum
class Shadowed
class Tagged
