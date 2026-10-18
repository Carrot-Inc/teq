//> using dep not.present::excluded:99
package shared

// Left out by the run's `--exclude` (the excluded-directive run): a dependency no lock holds, never read.
object Excluded
