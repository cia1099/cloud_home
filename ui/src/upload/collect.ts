import type { Picked } from './model'

/** Files from an `<input type=file>`. With `webkitdirectory` each file carries its path ("Photos/2024/a.jpg"). */
export function fromInput(files: FileList | null): Picked[] {
  return [...(files ?? [])].map((file) => ({ file, dirs: directoriesOf(file.webkitRelativePath) }))
}

/** "Photos/2024/a.jpg" → ["Photos", "2024"]; a plain file name has no directories. */
export function directoriesOf(relativePath: string): string[] {
  return relativePath.split('/').filter(Boolean).slice(0, -1)
}

const readAll = (reader: FileSystemDirectoryReader): Promise<FileSystemEntry[]> =>
  new Promise((resolve, reject) => {
    const all: FileSystemEntry[] = []
    // readEntries returns the directory in batches; an empty batch means the end.
    const next = () =>
      reader.readEntries((batch) => {
        if (batch.length === 0) resolve(all)
        else {
          all.push(...batch)
          next()
        }
      }, reject)
    next()
  })

const toFile = (entry: FileSystemFileEntry): Promise<File> => new Promise((resolve, reject) => entry.file(resolve, reject))

async function walk(entry: FileSystemEntry, dirs: string[], out: Picked[]): Promise<void> {
  if (entry.isFile) {
    out.push({ file: await toFile(entry as FileSystemFileEntry), dirs })
  } else if (entry.isDirectory) {
    const children = await readAll((entry as FileSystemDirectoryEntry).createReader())
    for (const child of children) await walk(child, [...dirs, entry.name], out)
  }
}

/**
 * Files and folders from a drop. The entries must be taken synchronously inside the drop event
 * (the list is cleared afterwards), so this reads them first and only then awaits.
 */
export async function fromDrop(items: DataTransferItemList): Promise<Picked[]> {
  const entries: (FileSystemEntry | null)[] = [...items].filter((item) => item.kind === 'file').map((item) => item.webkitGetAsEntry())
  const out: Picked[] = []
  for (const entry of entries) if (entry) await walk(entry, [], out)
  return out
}
