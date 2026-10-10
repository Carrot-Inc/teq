package sbt.internal.teq

import sbt.{Configuration, Def, ProjectRef}
import sbt.internal.{BuildDependencies, ClasspathImpl}

/** What sbt keeps to itself about the projects a configuration's classpath reaches, sbt 2's side (sbt 1's is
  * src/main/scala-sbt-1.0's). */
object InterDependencies {
  /** The (project, configuration) pairs whose products sbt puts on the classpath of `config`
    * (its `classpathConfiguration`) in sbt's order, the configuration itself and `self` left
    * out, as `internalDependencyClasspath` takes them. */
  def of(project: ProjectRef, config: Configuration, self: Configuration, data: Def.Settings, deps: BuildDependencies): Seq[(ProjectRef, String)] =
    ClasspathImpl.interSort(project, config, data, deps).filter { case (dep, c) => dep != project || (config.name != c && self.name != c) }
}
