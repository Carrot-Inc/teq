//> using dep com.softwaremill.macwire::macros:2.6.7
//> using dep com.softwaremill.macwire::util:2.6.7
// jars: macwire-macros macwire-util
// macwire's @Module: wire[T] takes the members of a value whose class carries the Java annotation, which
// reflection gives as a tree (Symbol.annotations) and whose fields a lookup on the value reads through its
// type (dotty's lookupPrefix of a term, its namedType), as lila's Envs take each other's members.
import com.softwaremill.macwire.*

class GameRepo:
  override def toString = "GameRepo"
class PgnDump(repo: GameRepo):
  override def toString = s"PgnDump($repo)"

@Module
final class GameEnv(name: String):
  lazy val gameRepo = new GameRepo

final class ApiEnv(gameEnv: GameEnv):
  lazy val pgnDump: PgnDump = wire[PgnDump]

object Main:
  def main(args: Array[String]): Unit =
    println(ApiEnv(GameEnv("g")).pgnDump)
