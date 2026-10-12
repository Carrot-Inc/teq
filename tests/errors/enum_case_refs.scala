// expect: 15:20: error: illegal reference to local from an enum case; write E.local
// expect: 16:20: error: illegal reference to twice from an enum case; write E.twice
// expect: 2 errors found
// An enum case's parent may name another case and what the scope around the enum reaches (an import
// of the companion's member outside the enum included), as dotty's checkEnumCaseRefsLegal allows,
// but no other member of the companion.
import E.shared

enum E(val xs: List[Int]):
  def this(x: Int) = this(List(x))
  case A extends E(1)
  case B extends E(A.xs)
  case C extends E(shared)
  case D extends E(List(B.xs.head, C.xs.head))
  case F extends E(local)
  case G extends E(twice(1))

object E:
  val shared = List(3)
  val local = List(4)
  def twice(x: Int) = List(x, x)
