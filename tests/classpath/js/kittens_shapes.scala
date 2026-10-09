// jars: cats-kernel-sjs cats-core-sjs shapeless3-deriving kittens
// teq: --max-inlines 80
//> using dep org.typelevel::kittens:3.5.0
//> using option -Xmax-inlines 80
// The shapes of the application's `derives` sites through kittens: an enum of 56 cases past
// the default inline limit, an enum mixing singleton cases with class cases whose fields are
// another enum, `Monoid` on summaries with a `BigDecimal`, `Eq` through a case class with a
// private constructor, an enum with a parameter, and class cases with defaults and empty
// parameter lists; the nested types derive without instances of their own.
import cats.*
import cats.derived.*
import cats.syntax.all.*

enum State derives Eq, Show:
  case AL, AK, AZ, AR, CA, CO, CT, DE, FL, GA, HI, ID, IL, IN, IA, KS, KY, LA, ME, MD, MA, MI, MN, MS, MO, MT, NE, NV, NH, NJ,
    NM, NY, NC, ND, OH, OK, OR, PA, RI, SC, SD, TN, TX, UT, VT, VA, WA, WV, WI, WY, DC, PR, GU, VI, AS, MP

final case class Loan(id: Int, items: List[String]) derives Show
enum Inner derives Show:
  case Changed(flag: Boolean)
  case NewLoan(loan: Loan, count: Int)
enum Event derives Show:
  case Ping
  case Branch(branchId: Int, value: Inner)
  case Library(value: Inner)

final case class Agg(key: String, value: Long)
final case class Summary(aggregations: Set[Agg], totalNotices: Long, totalCost: BigDecimal) derives Monoid
final case class Counts(sent: Long, cost: BigDecimal) derives Monoid, Eq

final case class Code private (sequential: Long, random: Int)
object Code:
  def of(s: Long, r: Int): Code = Code(s, r)
final case class LibraryCode(value: Code) derives Eq
enum Key(val entryName: String) derives Eq:
  case OTP extends Key("mail:otp")
  case Chat extends Key("mail:chat")
enum Elem derives Eq:
  case Genres
  case Picks
  case Top(genre: String, title: String)
enum Section derives Eq:
  case Activity(filter: Option[String])
  case Loans
enum Page derives Eq, CanEqual:
  case Reports()
  case Profile(id: Int, section: Section = Section.Activity(None))
enum FeeError derives Eq:
  case TierNotFound(titleId: Int, amount: BigDecimal)
  case TitleIssue(cause: Elem)

object Main:
  def main(args: Array[String]): Unit =
    println(show"${State.CA} ${State.MP} ${State.AL === State.AL} ${State.AL === State.AK}")
    println(show"${Event.Ping} ${Event.Branch(3, Inner.NewLoan(Loan(1, List("a")), 2))} ${Event.Library(Inner.Changed(true))}")
    println(Summary(Set(Agg("a", 1)), 2, BigDecimal("1.5")) |+| Summary(Set(Agg("b", 2)), 3, BigDecimal("2.5")))
    println(Monoid[Summary].empty)
    println((Counts(1, BigDecimal(2)) |+| Counts(3, BigDecimal(4))) === Counts(4, BigDecimal(6)))
    println(LibraryCode(Code.of(1, 2)) === LibraryCode(Code.of(1, 2)))
    println((Key.OTP === Key.Chat).toString + " " + (Key.OTP === Key.OTP) + " " + Key.Chat.entryName)
    println((Elem.Genres === Elem.Genres).toString + " " + ((Elem.Top("a", "b"): Elem) === Elem.Top("a", "b")) + " " + (Elem.Picks === Elem.Genres))
    println(((Page.Reports(): Page) === Page.Reports()).toString + " " + ((Page.Profile(1): Page) === Page.Profile(1)) + " " + ((Page.Profile(1): Page) === Page.Profile(1, Section.Loans)))
    println(((FeeError.TierNotFound(1, BigDecimal(2)): FeeError) === FeeError.TierNotFound(1, BigDecimal("2.0"))).toString + " " + ((FeeError.TitleIssue(Elem.Picks): FeeError) === FeeError.TitleIssue(Elem.Genres)))
