// A call that leaves out arguments with defaults, the default's getter applied to the receiver and the earlier
// arguments: one call per cell (where the method lives, what its default reads, the call's forms), every operand and
// default printing its tag when it is evaluated (generated; the order and values are scalac's).
object Log:
  val sb = new StringBuilder
  def e(tag: String, a: Int): Int = { sb.append(tag).append(' '); a }
  def o[A <: AnyRef](tag: String, a: A): A = { sb.append(tag).append(' '); a }
  def take(): String = { val s = sb.toString.trim; sb.clear(); s }
import Log.{e, o}
case class U(k: Int)
given gu: U = U(7)

object M009:
  val n = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

object M010:
  val n = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

object M011:
  val n = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x

object M012:
  val n = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x

object M013:
  val n = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

object M016:
  val n = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

object M017:
  val n = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

object M018:
  val n = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

val n038 = 10
def top038(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

val n039 = 10
def top039(a: Int)(x: Int = e("d", 7)): Int = a + x

val n040 = 10
def top040(a: Int)(x: Int = e("d", a + 1)): Int = a + x

val n043 = 10
def top043(a: => Int)(x: Int = e("d", 7)): Int = a + x

val n044 = 10
def top044(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

val n045 = 10
def top045(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

class C063(val n: Int):
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

class C064(val n: Int):
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

class C065(val n: Int):
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x

class C066(val n: Int):
  def f(a: Int)(x: Int = e("d", n)): Int = a + x

class C067(val n: Int):
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

class C070(val n: Int):
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

class C071(val n: Int):
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

class C072(val n: Int):
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

class C096(val n: Int):
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

class C097(val n: Int):
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

class C098(val n: Int):
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x

class C099(val n: Int):
  def f(a: Int)(x: Int = e("d", n)): Int = a + x

class C100(val n: Int):
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

class C103(val n: Int):
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

class C104(val n: Int):
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

class C105(val n: Int):
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

class C129(val n: Int):
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

class C130(val n: Int):
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

class C131(val n: Int):
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x

class C132(val n: Int):
  def f(a: Int)(x: Int = e("d", n)): Int = a + x

class C133(val n: Int):
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

class C136(val n: Int):
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

class C137(val n: Int):
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

class C138(val n: Int):
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

trait T162:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y
class S162 extends T162

trait T163:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y
class S163 extends T163

trait T164:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x
class S164 extends T164

trait T165:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x
class S165 extends T165

trait T166:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x
class S166 extends T166

trait T169:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x
class S169 extends T169

trait T170:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x
class S170 extends T170

trait T171:
  def n: Int = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x
class S171 extends T171

trait T195:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y
class S195 extends T195

trait T196:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y
class S196 extends T196

trait T197:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x
class S197 extends T197

trait T198:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x
class S198 extends T198

trait T199:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x
class S199 extends T199

trait T202:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x
class S202 extends T202

trait T203:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x
class S203 extends T203

trait T204:
  def n: Int = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x
class S204 extends T204

class G228[T](val t: T):
  val n = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

class G229[T](val t: T):
  val n = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

class G230[T](val t: T):
  val n = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x

class G231[T](val t: T):
  val n = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x

class G232[T](val t: T):
  val n = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

class G235[T](val t: T):
  val n = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

class G236[T](val t: T):
  val n = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

class G237[T](val t: T):
  val n = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

extension (s261: String) def ext261(a: Int, x: Int = e("d", 7), y: Int): Int = s261.length + a + x + y

extension (s262: String) def ext262(a: Int, x: Int = e("d", s262.length), y: Int): Int = s262.length + a + x + y

extension (s263: String) def ext263(a: Int)(x: Int = e("d", 7)): Int = s263.length + a + x

extension (s264: String) def ext264(a: Int)(x: Int = e("d", s264.length)): Int = s264.length + a + x

extension (s265: String) def ext265(a: Int)(x: Int = e("d", a + 1)): Int = s265.length + a + x

extension (s268: String) def ext268(a: => Int)(x: Int = e("d", 7)): Int = s268.length + a + x

extension (s269: String) def ext269(a: => Int)(x: Int = e("d", a + 1)): Int = s269.length + a + x

extension (s270: String) def ext270(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = s270.length + a + x

object X294:
  extension (s: String) def ext(a: Int, x: Int = e("d", 7), y: Int): Int = s.length + a + x + y

object X295:
  extension (s: String) def ext(a: Int, x: Int = e("d", s.length), y: Int): Int = s.length + a + x + y

object X296:
  extension (s: String) def ext(a: Int)(x: Int = e("d", 7)): Int = s.length + a + x

object X297:
  extension (s: String) def ext(a: Int)(x: Int = e("d", s.length)): Int = s.length + a + x

object X298:
  extension (s: String) def ext(a: Int)(x: Int = e("d", a + 1)): Int = s.length + a + x

object X301:
  extension (s: String) def ext(a: => Int)(x: Int = e("d", 7)): Int = s.length + a + x

object X302:
  extension (s: String) def ext(a: => Int)(x: Int = e("d", a + 1)): Int = s.length + a + x

object X303:
  extension (s: String) def ext(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = s.length + a + x

class V327(val w: Int) extends AnyVal:
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = w + a + x + y

class V328(val w: Int) extends AnyVal:
  def f(a: Int, x: Int = e("d", w), y: Int): Int = w + a + x + y

class V329(val w: Int) extends AnyVal:
  def f(a: Int)(x: Int = e("d", 7)): Int = w + a + x

class V330(val w: Int) extends AnyVal:
  def f(a: Int)(x: Int = e("d", w)): Int = w + a + x

class V331(val w: Int) extends AnyVal:
  def f(a: Int)(x: Int = e("d", a + 1)): Int = w + a + x

class V334(val w: Int) extends AnyVal:
  def f(a: => Int)(x: Int = e("d", 7)): Int = w + a + x

class V335(val w: Int) extends AnyVal:
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = w + a + x

class V336(val w: Int) extends AnyVal:
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = w + a + x









object N393:
  val n = 10
  inline def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y

object N394:
  val n = 10
  inline def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y

object N395:
  val n = 10
  inline def f(a: Int)(x: Int = e("d", 7)): Int = a + x

object N396:
  val n = 10
  inline def f(a: Int)(x: Int = e("d", n)): Int = a + x

object N397:
  val n = 10
  inline def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x

object N400:
  val n = 10
  inline def f(a: => Int)(x: Int = e("d", 7)): Int = a + x

object N401:
  val n = 10
  inline def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x

object N402:
  val n = 10
  inline def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x

class B426:
  val n = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y
class Sb426 extends B426:
  def run(): Int = super.f(y = e("y", 3), a = e("a", 1))

class B427:
  val n = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y
class Sb427 extends B427:
  def run(): Int = super.f(y = e("y", 3), a = e("a", 1))

class B428:
  val n = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x
class Sb428 extends B428:
  def run(): Int = super.f(e("a", 1))()

class B429:
  val n = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x
class Sb429 extends B429:
  def run(): Int = super.f(e("a", 1))()

class B430:
  val n = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x
class Sb430 extends B430:
  def run(): Int = super.f(e("a", 1))()

class B433:
  val n = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x
class Sb433 extends B433:
  def run(): Int = super.f(e("a", 1))()

class B434:
  val n = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x
class Sb434 extends B434:
  def run(): Int = super.f(e("a", 1))()

class B435:
  val n = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x
class Sb435 extends B435:
  def run(): Int = super.f()()

trait Tb459:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y
trait Ub459 extends Tb459:
  def run(): Int = super.f(y = e("y", 3), a = e("a", 1))
class Wb459 extends Ub459

trait Tb460:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y
trait Ub460 extends Tb460:
  def run(): Int = super.f(y = e("y", 3), a = e("a", 1))
class Wb460 extends Ub460

trait Tb461:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x
trait Ub461 extends Tb461:
  def run(): Int = super.f(e("a", 1))()
class Wb461 extends Ub461

trait Tb462:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x
trait Ub462 extends Tb462:
  def run(): Int = super.f(e("a", 1))()
class Wb462 extends Ub462

trait Tb463:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x
trait Ub463 extends Tb463:
  def run(): Int = super.f(e("a", 1))()
class Wb463 extends Ub463

trait Tb466:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x
trait Ub466 extends Tb466:
  def run(): Int = super.f(e("a", 1))()
class Wb466 extends Ub466

trait Tb467:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x
trait Ub467 extends Tb467:
  def run(): Int = super.f(e("a", 1))()
class Wb467 extends Ub467

trait Tb468:
  def n: Int = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x
trait Ub468 extends Tb468:
  def run(): Int = super.f()()
class Wb468 extends Ub468

trait Td492:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", 7), y: Int): Int = a + x + y
class Sd492 extends Td492:
  def run(): Int = f(y = e("y", 3), a = e("a", 1))

trait Td493:
  def n: Int = 10
  def f(a: Int, x: Int = e("d", n), y: Int): Int = a + x + y
class Sd493 extends Td493:
  def run(): Int = f(y = e("y", 3), a = e("a", 1))

trait Td494:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", 7)): Int = a + x
class Sd494 extends Td494:
  def run(): Int = f(e("a", 1))()

trait Td495:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", n)): Int = a + x
class Sd495 extends Td495:
  def run(): Int = f(e("a", 1))()

trait Td496:
  def n: Int = 10
  def f(a: Int)(x: Int = e("d", a + 1)): Int = a + x
class Sd496 extends Td496:
  def run(): Int = f(e("a", 1))()

trait Td499:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", 7)): Int = a + x
class Sd499 extends Td499:
  def run(): Int = f(e("a", 1))()

trait Td500:
  def n: Int = 10
  def f(a: => Int)(x: Int = e("d", a + 1)): Int = a + x
class Sd500 extends Td500:
  def run(): Int = f(e("a", 1))()

trait Td501:
  def n: Int = 10
  def f(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = a + x
class Sd501 extends Td501:
  def run(): Int = f()()

class K521(a: Int, x: Int = e("d", 7), y: Int):
  def v: Int = a + x + y

class K522(a: Int)(x: Int = e("d", 7)):
  def v: Int = a + x

class K523(a: Int)(x: Int = e("d", a + 1)):
  def v: Int = a + x

class K526(a: => Int)(x: Int = e("d", 7)):
  def v: Int = a + x

class K527(a: => Int)(x: Int = e("d", a + 1)):
  def v: Int = a + x

class K528(a: Int = e("da", 1))(x: Int = e("d", a + 1)):
  def v: Int = a + x

class O546:
  val n = 10
  class I(a: Int, x: Int = e("d", 7), y: Int):
    def v: Int = a + x + y

class O547:
  val n = 10
  class I(a: Int, x: Int = e("d", n), y: Int):
    def v: Int = a + x + y

class O548:
  val n = 10
  class I(a: Int)(x: Int = e("d", 7)):
    def v: Int = a + x

class O549:
  val n = 10
  class I(a: Int)(x: Int = e("d", n)):
    def v: Int = a + x

class O550:
  val n = 10
  class I(a: Int)(x: Int = e("d", a + 1)):
    def v: Int = a + x

class O553:
  val n = 10
  class I(a: => Int)(x: Int = e("d", 7)):
    def v: Int = a + x

class O554:
  val n = 10
  class I(a: => Int)(x: Int = e("d", a + 1)):
    def v: Int = a + x

class O555:
  val n = 10
  class I(a: Int = e("da", 1))(x: Int = e("d", a + 1)):
    def v: Int = a + x









case class P608(a: Int, x: Int = e("d", 7), y: Int):
  def v: Int = a + x + y

case class P609(a: Int)(x: Int = e("d", 7)):
  def v: Int = a + x

case class P610(a: Int)(x: Int = e("d", a + 1)):
  def v: Int = a + x

case class P613(a: Int = e("da", 1))(x: Int = e("d", a + 1)):
  def v: Int = a + x

class Q627(a: Int, x: Int = e("d", 7), y: Int):
  def v: Int = a + x + y
class E627 extends Q627(y = e("y", 3), a = e("a", 1))

class Q628(a: Int)(x: Int = e("d", 7)):
  def v: Int = a + x
class E628 extends Q628(e("a", 1))()

class Q629(a: Int)(x: Int = e("d", a + 1)):
  def v: Int = a + x
class E629 extends Q629(e("a", 1))()

class Q632(a: => Int)(x: Int = e("d", 7)):
  def v: Int = a + x
class E632 extends Q632(e("a", 1))()

class Q633(a: => Int)(x: Int = e("d", a + 1)):
  def v: Int = a + x
class E633 extends Q633(e("a", 1))()

class Q634(a: Int = e("da", 1))(x: Int = e("d", a + 1)):
  def v: Int = a + x
class E634 extends Q634()()

extension (s: String) def +%:(n: Int)(x: Int = e("d", s.length + n)): Int = s.length + n + x

def c009(): Int = M009.f(y = e("y", 3), a = e("a", 1))
def c010(): Int = M010.f(y = e("y", 3), a = e("a", 1))
def c011(): Int = M011.f(e("a", 1))()
def c012(): Int = M012.f(e("a", 1))()
def c013(): Int = M013.f(e("a", 1))()
def c016(): Int = M016.f(e("a", 1))()
def c017(): Int = M017.f(e("a", 1))()
def c018(): Int = M018.f()()
def c038(): Int = top038(y = e("y", 3), a = e("a", 1))
def c039(): Int = top039(e("a", 1))()
def c040(): Int = top040(e("a", 1))()
def c043(): Int = top043(e("a", 1))()
def c044(): Int = top044(e("a", 1))()
def c045(): Int = top045()()
def c063(): Int = { val c = new C063(10); c.f(y = e("y", 3), a = e("a", 1)) }
def c064(): Int = { val c = new C064(10); c.f(y = e("y", 3), a = e("a", 1)) }
def c065(): Int = { val c = new C065(10); c.f(e("a", 1))() }
def c066(): Int = { val c = new C066(10); c.f(e("a", 1))() }
def c067(): Int = { val c = new C067(10); c.f(e("a", 1))() }
def c070(): Int = { val c = new C070(10); c.f(e("a", 1))() }
def c071(): Int = { val c = new C071(10); c.f(e("a", 1))() }
def c072(): Int = { val c = new C072(10); c.f()() }
def c096(): Int = new C096(10).f(y = e("y", 3), a = e("a", 1))
def c097(): Int = new C097(10).f(y = e("y", 3), a = e("a", 1))
def c098(): Int = new C098(10).f(e("a", 1))()
def c099(): Int = new C099(10).f(e("a", 1))()
def c100(): Int = new C100(10).f(e("a", 1))()
def c103(): Int = new C103(10).f(e("a", 1))()
def c104(): Int = new C104(10).f(e("a", 1))()
def c105(): Int = new C105(10).f()()
def c129(): Int = o("r", new C129(10)).f(y = e("y", 3), a = e("a", 1))
def c130(): Int = o("r", new C130(10)).f(y = e("y", 3), a = e("a", 1))
def c131(): Int = o("r", new C131(10)).f(e("a", 1))()
def c132(): Int = o("r", new C132(10)).f(e("a", 1))()
def c133(): Int = o("r", new C133(10)).f(e("a", 1))()
def c136(): Int = o("r", new C136(10)).f(e("a", 1))()
def c137(): Int = o("r", new C137(10)).f(e("a", 1))()
def c138(): Int = o("r", new C138(10)).f()()
def c162(): Int = o("r", new S162).f(y = e("y", 3), a = e("a", 1))
def c163(): Int = o("r", new S163).f(y = e("y", 3), a = e("a", 1))
def c164(): Int = o("r", new S164).f(e("a", 1))()
def c165(): Int = o("r", new S165).f(e("a", 1))()
def c166(): Int = o("r", new S166).f(e("a", 1))()
def c169(): Int = o("r", new S169).f(e("a", 1))()
def c170(): Int = o("r", new S170).f(e("a", 1))()
def c171(): Int = o("r", new S171).f()()
def c195(): Int = { val t: T195 = new S195; o("r", t).f(y = e("y", 3), a = e("a", 1)) }
def c196(): Int = { val t: T196 = new S196; o("r", t).f(y = e("y", 3), a = e("a", 1)) }
def c197(): Int = { val t: T197 = new S197; o("r", t).f(e("a", 1))() }
def c198(): Int = { val t: T198 = new S198; o("r", t).f(e("a", 1))() }
def c199(): Int = { val t: T199 = new S199; o("r", t).f(e("a", 1))() }
def c202(): Int = { val t: T202 = new S202; o("r", t).f(e("a", 1))() }
def c203(): Int = { val t: T203 = new S203; o("r", t).f(e("a", 1))() }
def c204(): Int = { val t: T204 = new S204; o("r", t).f()() }
def c228(): Int = o("r", new G228("t")).f(y = e("y", 3), a = e("a", 1))
def c229(): Int = o("r", new G229("t")).f(y = e("y", 3), a = e("a", 1))
def c230(): Int = o("r", new G230("t")).f(e("a", 1))()
def c231(): Int = o("r", new G231("t")).f(e("a", 1))()
def c232(): Int = o("r", new G232("t")).f(e("a", 1))()
def c235(): Int = o("r", new G235("t")).f(e("a", 1))()
def c236(): Int = o("r", new G236("t")).f(e("a", 1))()
def c237(): Int = o("r", new G237("t")).f()()
def c261(): Int = o("s", "abc").ext261(y = e("y", 3), a = e("a", 1))
def c262(): Int = o("s", "abc").ext262(y = e("y", 3), a = e("a", 1))
def c263(): Int = o("s", "abc").ext263(e("a", 1))()
def c264(): Int = o("s", "abc").ext264(e("a", 1))()
def c265(): Int = o("s", "abc").ext265(e("a", 1))()
def c268(): Int = o("s", "abc").ext268(e("a", 1))()
def c269(): Int = o("s", "abc").ext269(e("a", 1))()
def c270(): Int = o("s", "abc").ext270()()
def c294(): Int = { import X294.*; o("s", "abc").ext(y = e("y", 3), a = e("a", 1)) }
def c295(): Int = { import X295.*; o("s", "abc").ext(y = e("y", 3), a = e("a", 1)) }
def c296(): Int = { import X296.*; o("s", "abc").ext(e("a", 1))() }
def c297(): Int = { import X297.*; o("s", "abc").ext(e("a", 1))() }
def c298(): Int = { import X298.*; o("s", "abc").ext(e("a", 1))() }
def c301(): Int = { import X301.*; o("s", "abc").ext(e("a", 1))() }
def c302(): Int = { import X302.*; o("s", "abc").ext(e("a", 1))() }
def c303(): Int = { import X303.*; o("s", "abc").ext()() }
def c327(): Int = new V327(e("v", 10)).f(y = e("y", 3), a = e("a", 1))
def c328(): Int = new V328(e("v", 10)).f(y = e("y", 3), a = e("a", 1))
def c329(): Int = new V329(e("v", 10)).f(e("a", 1))()
def c330(): Int = new V330(e("v", 10)).f(e("a", 1))()
def c331(): Int = new V331(e("v", 10)).f(e("a", 1))()
def c334(): Int = new V334(e("v", 10)).f(e("a", 1))()
def c335(): Int = new V335(e("v", 10)).f(e("a", 1))()
def c336(): Int = new V336(e("v", 10)).f()()
def c360(): Int = { val n = e("n", 10); def loc(a: Int, x: Int = e("d", 7), y: Int): Int = n + a + x + y; loc(y = e("y", 3), a = e("a", 1)) }
def c361(): Int = { val n = e("n", 10); def loc(a: Int, x: Int = e("d", n), y: Int): Int = n + a + x + y; loc(y = e("y", 3), a = e("a", 1)) }
def c362(): Int = { val n = e("n", 10); def loc(a: Int)(x: Int = e("d", 7)): Int = n + a + x; loc(e("a", 1))() }
def c363(): Int = { val n = e("n", 10); def loc(a: Int)(x: Int = e("d", n)): Int = n + a + x; loc(e("a", 1))() }
def c364(): Int = { val n = e("n", 10); def loc(a: Int)(x: Int = e("d", a + 1)): Int = n + a + x; loc(e("a", 1))() }
def c367(): Int = { val n = e("n", 10); def loc(a: => Int)(x: Int = e("d", 7)): Int = n + a + x; loc(e("a", 1))() }
def c368(): Int = { val n = e("n", 10); def loc(a: => Int)(x: Int = e("d", a + 1)): Int = n + a + x; loc(e("a", 1))() }
def c369(): Int = { val n = e("n", 10); def loc(a: Int = e("da", 1))(x: Int = e("d", a + 1)): Int = n + a + x; loc()() }
def c393(): Int = N393.f(y = e("y", 3), a = e("a", 1))
def c394(): Int = N394.f(y = e("y", 3), a = e("a", 1))
def c395(): Int = N395.f(e("a", 1))()
def c396(): Int = N396.f(e("a", 1))()
def c397(): Int = N397.f(e("a", 1))()
def c400(): Int = N400.f(e("a", 1))()
def c401(): Int = N401.f(e("a", 1))()
def c402(): Int = N402.f()()
def c426(): Int = new Sb426().run()
def c427(): Int = new Sb427().run()
def c428(): Int = new Sb428().run()
def c429(): Int = new Sb429().run()
def c430(): Int = new Sb430().run()
def c433(): Int = new Sb433().run()
def c434(): Int = new Sb434().run()
def c435(): Int = new Sb435().run()
def c459(): Int = new Wb459().run()
def c460(): Int = new Wb460().run()
def c461(): Int = new Wb461().run()
def c462(): Int = new Wb462().run()
def c463(): Int = new Wb463().run()
def c466(): Int = new Wb466().run()
def c467(): Int = new Wb467().run()
def c468(): Int = new Wb468().run()
def c492(): Int = new Sd492().run()
def c493(): Int = new Sd493().run()
def c494(): Int = new Sd494().run()
def c495(): Int = new Sd495().run()
def c496(): Int = new Sd496().run()
def c499(): Int = new Sd499().run()
def c500(): Int = new Sd500().run()
def c501(): Int = new Sd501().run()
def c521(): Int = new K521(y = e("y", 3), a = e("a", 1)).v
def c522(): Int = new K522(e("a", 1))().v
def c523(): Int = new K523(e("a", 1))().v
def c526(): Int = new K526(e("a", 1))().v
def c527(): Int = new K527(e("a", 1))().v
def c528(): Int = new K528()().v
def c546(): Int = { val o1 = o("o", new O546); new o1.I(y = e("y", 3), a = e("a", 1)).v }
def c547(): Int = { val o1 = o("o", new O547); new o1.I(y = e("y", 3), a = e("a", 1)).v }
def c548(): Int = { val o1 = o("o", new O548); new o1.I(e("a", 1))().v }
def c549(): Int = { val o1 = o("o", new O549); new o1.I(e("a", 1))().v }
def c550(): Int = { val o1 = o("o", new O550); new o1.I(e("a", 1))().v }
def c553(): Int = { val o1 = o("o", new O553); new o1.I(e("a", 1))().v }
def c554(): Int = { val o1 = o("o", new O554); new o1.I(e("a", 1))().v }
def c555(): Int = { val o1 = o("o", new O555); new o1.I()().v }
def c579(): Int = { val n = e("n", 10); class L(a: Int, x: Int = e("d", 7), y: Int) { def v: Int = n + a + x + y }; new L(y = e("y", 3), a = e("a", 1)).v }
def c580(): Int = { val n = e("n", 10); class L(a: Int, x: Int = e("d", n), y: Int) { def v: Int = n + a + x + y }; new L(y = e("y", 3), a = e("a", 1)).v }
def c581(): Int = { val n = e("n", 10); class L(a: Int)(x: Int = e("d", 7)) { def v: Int = n + a + x }; new L(e("a", 1))().v }
def c582(): Int = { val n = e("n", 10); class L(a: Int)(x: Int = e("d", n)) { def v: Int = n + a + x }; new L(e("a", 1))().v }
def c583(): Int = { val n = e("n", 10); class L(a: Int)(x: Int = e("d", a + 1)) { def v: Int = n + a + x }; new L(e("a", 1))().v }
def c586(): Int = { val n = e("n", 10); class L(a: => Int)(x: Int = e("d", 7)) { def v: Int = n + a + x }; new L(e("a", 1))().v }
def c587(): Int = { val n = e("n", 10); class L(a: => Int)(x: Int = e("d", a + 1)) { def v: Int = n + a + x }; new L(e("a", 1))().v }
def c588(): Int = { val n = e("n", 10); class L(a: Int = e("da", 1))(x: Int = e("d", a + 1)) { def v: Int = n + a + x }; new L()().v }
def c608(): Int = P608(y = e("y", 3), a = e("a", 1)).v
def c609(): Int = P609(e("a", 1))().v
def c610(): Int = P610(e("a", 1))().v
def c613(): Int = P613()().v
def c627(): Int = new E627().v
def c628(): Int = new E628().v
def c629(): Int = new E629().v
def c632(): Int = new E632().v
def c633(): Int = new E633().v
def c634(): Int = new E634().v
def c648(): Int = +%:(e("n", 1))(o("s", "ab"))()

@main def m(): Unit =
  { val v = c009(); println(s"c009: ${Log.take()} = $v") }
  { val v = c010(); println(s"c010: ${Log.take()} = $v") }
  { val v = c011(); println(s"c011: ${Log.take()} = $v") }
  { val v = c012(); println(s"c012: ${Log.take()} = $v") }
  { val v = c013(); println(s"c013: ${Log.take()} = $v") }
  { val v = c016(); println(s"c016: ${Log.take()} = $v") }
  { val v = c017(); println(s"c017: ${Log.take()} = $v") }
  { val v = c018(); println(s"c018: ${Log.take()} = $v") }
  { val v = c038(); println(s"c038: ${Log.take()} = $v") }
  { val v = c039(); println(s"c039: ${Log.take()} = $v") }
  { val v = c040(); println(s"c040: ${Log.take()} = $v") }
  { val v = c043(); println(s"c043: ${Log.take()} = $v") }
  { val v = c044(); println(s"c044: ${Log.take()} = $v") }
  { val v = c045(); println(s"c045: ${Log.take()} = $v") }
  { val v = c063(); println(s"c063: ${Log.take()} = $v") }
  { val v = c064(); println(s"c064: ${Log.take()} = $v") }
  { val v = c065(); println(s"c065: ${Log.take()} = $v") }
  { val v = c066(); println(s"c066: ${Log.take()} = $v") }
  { val v = c067(); println(s"c067: ${Log.take()} = $v") }
  { val v = c070(); println(s"c070: ${Log.take()} = $v") }
  { val v = c071(); println(s"c071: ${Log.take()} = $v") }
  { val v = c072(); println(s"c072: ${Log.take()} = $v") }
  { val v = c096(); println(s"c096: ${Log.take()} = $v") }
  { val v = c097(); println(s"c097: ${Log.take()} = $v") }
  { val v = c098(); println(s"c098: ${Log.take()} = $v") }
  { val v = c099(); println(s"c099: ${Log.take()} = $v") }
  { val v = c100(); println(s"c100: ${Log.take()} = $v") }
  { val v = c103(); println(s"c103: ${Log.take()} = $v") }
  { val v = c104(); println(s"c104: ${Log.take()} = $v") }
  { val v = c105(); println(s"c105: ${Log.take()} = $v") }
  { val v = c129(); println(s"c129: ${Log.take()} = $v") }
  { val v = c130(); println(s"c130: ${Log.take()} = $v") }
  { val v = c131(); println(s"c131: ${Log.take()} = $v") }
  { val v = c132(); println(s"c132: ${Log.take()} = $v") }
  { val v = c133(); println(s"c133: ${Log.take()} = $v") }
  { val v = c136(); println(s"c136: ${Log.take()} = $v") }
  { val v = c137(); println(s"c137: ${Log.take()} = $v") }
  { val v = c138(); println(s"c138: ${Log.take()} = $v") }
  { val v = c162(); println(s"c162: ${Log.take()} = $v") }
  { val v = c163(); println(s"c163: ${Log.take()} = $v") }
  { val v = c164(); println(s"c164: ${Log.take()} = $v") }
  { val v = c165(); println(s"c165: ${Log.take()} = $v") }
  { val v = c166(); println(s"c166: ${Log.take()} = $v") }
  { val v = c169(); println(s"c169: ${Log.take()} = $v") }
  { val v = c170(); println(s"c170: ${Log.take()} = $v") }
  { val v = c171(); println(s"c171: ${Log.take()} = $v") }
  { val v = c195(); println(s"c195: ${Log.take()} = $v") }
  { val v = c196(); println(s"c196: ${Log.take()} = $v") }
  { val v = c197(); println(s"c197: ${Log.take()} = $v") }
  { val v = c198(); println(s"c198: ${Log.take()} = $v") }
  { val v = c199(); println(s"c199: ${Log.take()} = $v") }
  { val v = c202(); println(s"c202: ${Log.take()} = $v") }
  { val v = c203(); println(s"c203: ${Log.take()} = $v") }
  { val v = c204(); println(s"c204: ${Log.take()} = $v") }
  { val v = c228(); println(s"c228: ${Log.take()} = $v") }
  { val v = c229(); println(s"c229: ${Log.take()} = $v") }
  { val v = c230(); println(s"c230: ${Log.take()} = $v") }
  { val v = c231(); println(s"c231: ${Log.take()} = $v") }
  { val v = c232(); println(s"c232: ${Log.take()} = $v") }
  { val v = c235(); println(s"c235: ${Log.take()} = $v") }
  { val v = c236(); println(s"c236: ${Log.take()} = $v") }
  { val v = c237(); println(s"c237: ${Log.take()} = $v") }
  { val v = c261(); println(s"c261: ${Log.take()} = $v") }
  { val v = c262(); println(s"c262: ${Log.take()} = $v") }
  { val v = c263(); println(s"c263: ${Log.take()} = $v") }
  { val v = c264(); println(s"c264: ${Log.take()} = $v") }
  { val v = c265(); println(s"c265: ${Log.take()} = $v") }
  { val v = c268(); println(s"c268: ${Log.take()} = $v") }
  { val v = c269(); println(s"c269: ${Log.take()} = $v") }
  { val v = c270(); println(s"c270: ${Log.take()} = $v") }
  { val v = c294(); println(s"c294: ${Log.take()} = $v") }
  { val v = c295(); println(s"c295: ${Log.take()} = $v") }
  { val v = c296(); println(s"c296: ${Log.take()} = $v") }
  { val v = c297(); println(s"c297: ${Log.take()} = $v") }
  { val v = c298(); println(s"c298: ${Log.take()} = $v") }
  { val v = c301(); println(s"c301: ${Log.take()} = $v") }
  { val v = c302(); println(s"c302: ${Log.take()} = $v") }
  { val v = c303(); println(s"c303: ${Log.take()} = $v") }
  { val v = c327(); println(s"c327: ${Log.take()} = $v") }
  { val v = c328(); println(s"c328: ${Log.take()} = $v") }
  { val v = c329(); println(s"c329: ${Log.take()} = $v") }
  { val v = c330(); println(s"c330: ${Log.take()} = $v") }
  { val v = c331(); println(s"c331: ${Log.take()} = $v") }
  { val v = c334(); println(s"c334: ${Log.take()} = $v") }
  { val v = c335(); println(s"c335: ${Log.take()} = $v") }
  { val v = c336(); println(s"c336: ${Log.take()} = $v") }
  { val v = c360(); println(s"c360: ${Log.take()} = $v") }
  { val v = c361(); println(s"c361: ${Log.take()} = $v") }
  { val v = c362(); println(s"c362: ${Log.take()} = $v") }
  { val v = c363(); println(s"c363: ${Log.take()} = $v") }
  { val v = c364(); println(s"c364: ${Log.take()} = $v") }
  { val v = c367(); println(s"c367: ${Log.take()} = $v") }
  { val v = c368(); println(s"c368: ${Log.take()} = $v") }
  { val v = c369(); println(s"c369: ${Log.take()} = $v") }
  { val v = c393(); println(s"c393: ${Log.take()} = $v") }
  { val v = c394(); println(s"c394: ${Log.take()} = $v") }
  { val v = c395(); println(s"c395: ${Log.take()} = $v") }
  { val v = c396(); println(s"c396: ${Log.take()} = $v") }
  { val v = c397(); println(s"c397: ${Log.take()} = $v") }
  { val v = c400(); println(s"c400: ${Log.take()} = $v") }
  { val v = c401(); println(s"c401: ${Log.take()} = $v") }
  { val v = c402(); println(s"c402: ${Log.take()} = $v") }
  { val v = c426(); println(s"c426: ${Log.take()} = $v") }
  { val v = c427(); println(s"c427: ${Log.take()} = $v") }
  { val v = c428(); println(s"c428: ${Log.take()} = $v") }
  { val v = c429(); println(s"c429: ${Log.take()} = $v") }
  { val v = c430(); println(s"c430: ${Log.take()} = $v") }
  { val v = c433(); println(s"c433: ${Log.take()} = $v") }
  { val v = c434(); println(s"c434: ${Log.take()} = $v") }
  { val v = c435(); println(s"c435: ${Log.take()} = $v") }
  { val v = c459(); println(s"c459: ${Log.take()} = $v") }
  { val v = c460(); println(s"c460: ${Log.take()} = $v") }
  { val v = c461(); println(s"c461: ${Log.take()} = $v") }
  { val v = c462(); println(s"c462: ${Log.take()} = $v") }
  { val v = c463(); println(s"c463: ${Log.take()} = $v") }
  { val v = c466(); println(s"c466: ${Log.take()} = $v") }
  { val v = c467(); println(s"c467: ${Log.take()} = $v") }
  { val v = c468(); println(s"c468: ${Log.take()} = $v") }
  { val v = c492(); println(s"c492: ${Log.take()} = $v") }
  { val v = c493(); println(s"c493: ${Log.take()} = $v") }
  { val v = c494(); println(s"c494: ${Log.take()} = $v") }
  { val v = c495(); println(s"c495: ${Log.take()} = $v") }
  { val v = c496(); println(s"c496: ${Log.take()} = $v") }
  { val v = c499(); println(s"c499: ${Log.take()} = $v") }
  { val v = c500(); println(s"c500: ${Log.take()} = $v") }
  { val v = c501(); println(s"c501: ${Log.take()} = $v") }
  { val v = c521(); println(s"c521: ${Log.take()} = $v") }
  { val v = c522(); println(s"c522: ${Log.take()} = $v") }
  { val v = c523(); println(s"c523: ${Log.take()} = $v") }
  { val v = c526(); println(s"c526: ${Log.take()} = $v") }
  { val v = c527(); println(s"c527: ${Log.take()} = $v") }
  { val v = c528(); println(s"c528: ${Log.take()} = $v") }
  { val v = c546(); println(s"c546: ${Log.take()} = $v") }
  { val v = c547(); println(s"c547: ${Log.take()} = $v") }
  { val v = c548(); println(s"c548: ${Log.take()} = $v") }
  { val v = c549(); println(s"c549: ${Log.take()} = $v") }
  { val v = c550(); println(s"c550: ${Log.take()} = $v") }
  { val v = c553(); println(s"c553: ${Log.take()} = $v") }
  { val v = c554(); println(s"c554: ${Log.take()} = $v") }
  { val v = c555(); println(s"c555: ${Log.take()} = $v") }
  { val v = c579(); println(s"c579: ${Log.take()} = $v") }
  { val v = c580(); println(s"c580: ${Log.take()} = $v") }
  { val v = c581(); println(s"c581: ${Log.take()} = $v") }
  { val v = c582(); println(s"c582: ${Log.take()} = $v") }
  { val v = c583(); println(s"c583: ${Log.take()} = $v") }
  { val v = c586(); println(s"c586: ${Log.take()} = $v") }
  { val v = c587(); println(s"c587: ${Log.take()} = $v") }
  { val v = c588(); println(s"c588: ${Log.take()} = $v") }
  { val v = c608(); println(s"c608: ${Log.take()} = $v") }
  { val v = c609(); println(s"c609: ${Log.take()} = $v") }
  { val v = c610(); println(s"c610: ${Log.take()} = $v") }
  { val v = c613(); println(s"c613: ${Log.take()} = $v") }
  { val v = c627(); println(s"c627: ${Log.take()} = $v") }
  { val v = c628(); println(s"c628: ${Log.take()} = $v") }
  { val v = c629(); println(s"c629: ${Log.take()} = $v") }
  { val v = c632(); println(s"c632: ${Log.take()} = $v") }
  { val v = c633(); println(s"c633: ${Log.take()} = $v") }
  { val v = c634(); println(s"c634: ${Log.take()} = $v") }
  { val v = c648(); println(s"c648: ${Log.take()} = $v") }
