//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros macwire-util
// A local def with a declared result inside a member whose type is inferred (lila's security Env,
// `lazy val garbageCollector`): its `wire` waits for the member's expansion once the member's type
// is set, as dotty expands a plain inline call after typer (`Inlines.needsInlining`), so macwire's
// walk of the class reads the member's type, not the recursion.
import com.softwaremill.macwire.*
class UserLoginsApi
class GeoIP
class IpTrust(geo: GeoIP)
class NoteApi
class UserRepo
class Setting:
  def get(): Boolean = true
final class GarbageCollector(userLogins: UserLoginsApi, ipTrust: IpTrust, noteApi: NoteApi, isArmed: () => Boolean, userRepo: UserRepo)
final class Cli(userRepo: UserRepo, gc: GarbageCollector)
class Env(noteApi: NoteApi, userRepo: UserRepo):
  lazy val geoIP: GeoIP = wire[GeoIP]
  lazy val userLogins = wire[UserLoginsApi]
  lazy val ugcArmedSetting = new Setting
  lazy val garbageCollector =
    def mk: (() => Boolean) => GarbageCollector = isArmed => wire[GarbageCollector]
    mk(() => ugcArmedSetting.get())
  lazy val ipTrust: IpTrust = wire[IpTrust]
  wire[Cli]

@main def run(): Unit =
  println(Env(NoteApi(), UserRepo()).garbageCollector.getClass.getSimpleName)
