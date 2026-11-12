//> using options -unchecked:false
// -unchecked (ScalaSettings.scala:116, true by default) set false: scalac summarizes the unchecked warnings
// (UncheckedWarning.enablingOption, ConditionalWarning.isSummarizedConditional, Reporter.summarizeUnreportedWarnings);
// teq is to summarize them too.
def f(x: Any): Boolean = x.isInstanceOf[List[String]]
@main def run(): Unit = println(f(1))
