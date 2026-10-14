// What Airstream's generators take of sbt's API (their `import sbt._`): a path's child, sbt's `RichFile./`.
package sbt

import java.io.File

extension (file: File) def /(component: String): File = new File(file, component)
