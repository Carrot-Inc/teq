object Enabled:
  import scala.language.dynamics
  class Base extends Dynamic
class Sub extends Enabled.Base
class Direct extends Dynamic
trait Mixed extends scala.Dynamic
object Single extends Dynamic
// A class that derives from `scala.Dynamic` where the feature `dynamics` is not enabled (`Typer.typedClassDef`): an
// error where `scala.Dynamic` itself is one of its parents, a feature warning where it comes through another class.
// teq: --feature
// expect: 4:7: warning: extension of type scala.Dynamic should be enabled
// expect: why the feature should be explicitly enabled.
// expect: 5:7: error: extension of type scala.Dynamic needs to be enabled
// expect: by adding the import clause 'import scala.language.dynamics'
// expect: why the feature needs to be explicitly enabled.
// expect: 6:7: error: extension of type scala.Dynamic needs to be enabled
// expect: 7:8: error: extension of type scala.Dynamic needs to be enabled
// expect: 3 errors found
// absent: 3:9: error:
// absent: 3:9: warning:
