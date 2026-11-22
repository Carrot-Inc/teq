// A type test of `Any` or `Matchable` takes everything but `null`, as dotty's erasure writes it (`instanceof Object`):
// `isInstanceOf` in a condition, a block's result, under `!` and `&&`, as a value, as a member of a union or an
// intersection, an inline body's test or pattern of a type parameter given `Any`, and a pattern on a scrutinee typed
// `Any`. Only a pattern on a scrutinee whose type conforms to `Object` folds the test: to the test that it is not
// `null` (a `String`, an `AnyRef`), or to none (an `Int`). teq answered `true` for `null` on every target but in
// JavaScript's patterns written in the program. The cases after a test of `Any` or `Matchable` follow a guarded one:
// teq's reachability warnings take them for unreachable, where scalac warns nothing and the case runs for `null`.
def g(): Unit = ()
def yes(): Boolean = true
def isAny(x: Any): Boolean = x.isInstanceOf[Any]
def isMatchable(x: Any): Boolean = x.isInstanceOf[Matchable]
def stringIsAny(s: String): Boolean = s.isInstanceOf[Any]
def isUnion(x: Any): Boolean = x.isInstanceOf[Any | String]
def isUnion2(x: Any): Boolean = x.isInstanceOf[Int | Matchable]
def isInter(x: Any): Boolean = x.isInstanceOf[Matchable & CharSequence]
def ifAny(x: Any): Int = if (x.isInstanceOf[Any]) 1 else 2
def ifBlockAny(x: Any): Int = if ({ g(); x.isInstanceOf[Any] }) 1 else 2
def ifNotAny(x: Any): Int = if (!x.isInstanceOf[Any]) 1 else 2
def ifAndAny(x: Any, b: Boolean): Int = if (b && x.isInstanceOf[Any]) 1 else 2
inline def isA[T](x: Any): Boolean = x.isInstanceOf[T]
inline def caseA[T](x: Any): Int = x match { case _: T => 1; case _ => 2 }
inline def stringCaseA[T](s: String): Int = s match { case _: T => 1; case _ => 2 }
inline def intCaseA[T](i: Int): Int = i match { case _: T => 1; case _ => 2 }
def caseMatchable(x: Any): Int = x match { case _: Matchable if yes() => 1; case _ => 2 }
def caseAny(x: Any): Int = x match { case _: Any => 1 }
def caseAnyOrNull(x: Any): Int = x match { case _: Any if yes() => 1; case null => 3 }
def stringCaseAny(s: String): Int = s match { case _: Any if yes() => 1; case null => 2 }
def anyRefCaseAny(r: AnyRef): Int = r match { case _: Any if yes() => 1; case null => 2 }
def stringCaseMatchable(s: String): Int = s match { case _: Matchable if yes() => 1; case null => 2 }
def intCaseAny(i: Int): Int = i match { case _: Any => 1 }
def t(f: => Any): String = try f.toString catch { case _: MatchError => "MatchError" }
@main def run(): Unit =
  for (x, k) <- List[Any](null, "s", 5, ()).zipWithIndex do
    println(s"$k: isAny=${isAny(x)} isMatchable=${isMatchable(x)} isUnion=${isUnion(x)} isUnion2=${isUnion2(x)} isInter=${isInter(x)} isA[Any]=${isA[Any](x)} isA[Matchable]=${isA[Matchable](x)}")
    println(s"$k: ifAny=${ifAny(x)} ifBlockAny=${ifBlockAny(x)} ifNotAny=${ifNotAny(x)} ifAndAny=${ifAndAny(x, true)}")
    println(s"$k: caseMatchable=${t(caseMatchable(x))} caseAny=${t(caseAny(x))} caseAnyOrNull=${t(caseAnyOrNull(x))} caseA[Any]=${caseA[Any](x)} caseA[Matchable]=${caseA[Matchable](x)}")
  for (s, k) <- List[String](null, "s").zipWithIndex do
    println(s"$k: stringIsAny=${stringIsAny(s)} stringCaseAny=${stringCaseAny(s)} anyRefCaseAny=${anyRefCaseAny(s)} stringCaseMatchable=${stringCaseMatchable(s)} stringCaseA[Any]=${stringCaseA[Any](s)} stringCaseA[Matchable]=${stringCaseA[Matchable](s)}")
  println(s"intCaseAny=${intCaseAny(3)} intCaseA[Any]=${intCaseA[Any](3)}")
