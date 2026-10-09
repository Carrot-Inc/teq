final class PolicyType[T](val sqlTypeName: String, val fromString: String => Option[T], val toStr: T => String)

object PolicyType:
  given PolicyType[String] = new PolicyType[String]("String", s => Some(s), s => s)
  given PolicyType[Int] = new PolicyType[Int]("Int", s => s.toIntOption, i => i.toString)
  given PolicyType[Boolean] = new PolicyType[Boolean]("Boolean", s => if s == "true" then Some(true) else if s == "false" then Some(false) else None, b => b.toString)
  given [T](using inner: PolicyType[T]): PolicyType[Option[T]] = new PolicyType[Option[T]](
    sqlTypeName = s"Option[${inner.sqlTypeName}]",
    fromString = s => if s == "" then Some(None) else inner.fromString(s).map(Some(_)),
    toStr = o => o.map(inner.toStr).getOrElse(""),
  )

trait PolicyBase[T]:
  def key: String
  def default: T
  def description: String
  def policyType: PolicyType[T]
  def defaultText: String = policyType.toStr(default)

enum BranchPolicy[T](
  val key: String,
  val default: T,
  val description: String,
)(using val policyType: PolicyType[T]) extends PolicyBase[T]:
  case General_DeskPhone extends BranchPolicy[String](
    key            = "general:desk_phone",
    default        = "",
    description    = "Desk contact phone for members",
  )

  case Holds_MaxCount extends BranchPolicy[Int](
    key            = "holds:max_count",
    default        = 25,
    description    = "Maximum holds per member",
  )

  case System_LargeText extends BranchPolicy[Boolean](
    key            = "system:large_text",
    default        = false,
    description    = "Large text",
  )

  case FineRounding extends BranchPolicy[Option[Int]](
    key            = "fine_rounding",
    default        = None,
    description    = "Rounding",
  )

enum LibraryPolicy[T: PolicyType](val key: String, val default: T, val description: String) extends PolicyBase[T]:
  case NoticeFromName extends LibraryPolicy[String]("notice:from_name", "Acme", "Sender name")
  case MaxDesks extends LibraryPolicy[Int]("desks:max", 10, "Desk limit")

  def policyType: PolicyType[T] = summon[PolicyType[T]]

enum Policy[T]:
  case Library(value: LibraryPolicy[T])
  case Branch(value: BranchPolicy[T])

  def key: String = this match
    case Library(value) => value.key
    case Branch(value) => value.key

  def defaultText: String = this match
    case Library(value) => value.policyType.toStr(value.default)
    case Branch(value) => value.policyType.toStr(value.default)

def getPolicy[T](policies: Map[String, String])(policy: Policy[T])(using st: PolicyType[T]): T =
  val (key, default) = policy match
    case Policy.Library(s) => (s.key, s.default)
    case Policy.Branch(s) => (s.key, s.default)
  policies.get(key).flatMap(st.fromString).getOrElse(default)

def useBranchPolicy[T](policies: Map[String, String], policy: BranchPolicy[T])(using PolicyType[T]): T =
  getPolicy(policies)(Policy.Branch(policy))

def widget(policy: BranchPolicy[?]): String = policy match
  case BranchPolicy.General_DeskPhone => "text"
  case BranchPolicy.Holds_MaxCount => "int"
  case BranchPolicy.System_LargeText => "boolean"
  case BranchPolicy.FineRounding => "parameterized"

@main def main(): Unit =
  val stored = Map("holds:max_count" -> "40", "system:large_text" -> "true", "desks:max" -> "oops")
  val maxHolds: Int = useBranchPolicy(stored, BranchPolicy.Holds_MaxCount)
  println(maxHolds + 1)
  val large: Boolean = useBranchPolicy(stored, BranchPolicy.System_LargeText)
  println(!large)
  println(useBranchPolicy(stored, BranchPolicy.General_DeskPhone).isEmpty)
  println(useBranchPolicy(stored, BranchPolicy.FineRounding))
  println(getPolicy(stored)(Policy.Library(LibraryPolicy.MaxDesks)))
  println(getPolicy(stored)(Policy.Library(LibraryPolicy.NoticeFromName)))
  for s <- BranchPolicy.values do
    println(s"${s.key} [${s.policyType.sqlTypeName}] = '${s.defaultText}' (${s.description}) ${widget(s)}")
  for s <- LibraryPolicy.values do
    println(s"${s.key} [${s.policyType.sqlTypeName}] = '${s.defaultText}'")
  val base: PolicyBase[Int] = BranchPolicy.Holds_MaxCount
  println(base.key)
  println(base.defaultText)
  println(Policy.Branch(BranchPolicy.Holds_MaxCount).defaultText)
  println(Policy.Library(LibraryPolicy.NoticeFromName).key)
  println(BranchPolicy.valueOf("FineRounding").ordinal)
