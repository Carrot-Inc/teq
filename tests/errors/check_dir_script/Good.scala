// A directory's script is one of its sources under the compiler's check, wrapped as teq interp wraps it (Scala
// CLI's ElementsUtils.singleFilesFromDirectory takes both), its error at the script's own line and column.
object Good
// expect: broken.sc:1:17: error: type mismatch: found 123, required String
