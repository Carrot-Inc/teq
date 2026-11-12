package rules
import teq.fix.*
case class Settings(names: List[String] = Nil, strict: Boolean = false, limit: Int = 3, pattern: Option[scala.util.matching.Regex] = None)
object Settings:
  val default: Settings = Settings()
  def read(conf: Conf): Configured[Settings] =
    for
      names <- conf.strings("names", default.names)
      strict <- conf.boolean("strict", default.strict)
      limit <- conf.int("limit", default.limit)
    yield Settings(names, strict, limit, None)
final class SettingsCaseDefault(settings: Settings) extends SyntacticRule("SettingsCaseDefault"):
  def this() = this(Settings.default)
  override def withConfiguration(c: Configuration): Configured[Rule] =
    Settings.read(c.conf.section("SettingsCaseDefault")).map(new SettingsCaseDefault(_))
  override def fix(using doc: SyntacticDocument): Patch =
    Patch.lint(Diagnostic("n", (settings.limit).toString, doc.tree.pos))
