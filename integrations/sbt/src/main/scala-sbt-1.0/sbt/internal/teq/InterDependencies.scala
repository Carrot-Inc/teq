package sbt.internal.teq

import sbt.{Configuration, ProjectRef, Scope}
import sbt.internal.{BuildDependencies, ClasspathImpl}
import sbt.internal.util.Settings

/** What sbt keeps to itself about the projects a configuration's classpath reaches, sbt 1's side (sbt 2's is
  * src/main/scala-sbt-2's): the same, over sbt 1's settings. */
object InterDependencies {
  /** The (project, configuration) pairs whose products sbt puts on the classpath of `config`
    * (its `classpathConfiguration`) in sbt's order, the configuration itself and `self` left
    * out, as `internalDependencyClasspath` takes them. */
  def of(project: ProjectRef, config: Configuration, self: Configuration, data: Settings[Scope], deps: BuildDependencies): Seq[(ProjectRef, String)] =
    ClasspathImpl.interSort(project, config, data, deps).filter { case (dep, c) => dep != project || (config.name != c && self.name != c) }
}
