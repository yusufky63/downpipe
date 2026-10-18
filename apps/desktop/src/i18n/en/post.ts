// After a download completes: the virus scan, extracting archives and a
// command, in the dialogs, the details, the list and Settings.
// {path}, {name}, {folder} and {url} in hints are the command placeholders
// themselves and stay as written.
export const post = {
  /** The step running now, as the list's status badge. */
  steps: {
    scanning: "Scanning",
    extracting: "Extracting",
    running_command: "Running command",
  },
  /** Badge of a download the virus scan flagged. */
  threat: "Threat",
  summary: {
    threat: "Threat found: {threat}",
    notExtracted: "Not extracted",
    commandFailed: "The command after download failed",
  },
  /** The details dialog's section. */
  section: "After download",
  scan: {
    label: "Virus scan",
    clean: "{scanner} found nothing.",
    threat:
      "{scanner} found {threat}. Nothing was extracted or run, and the file was left as it is: deleting it is up to you.",
    /** Stands for {threat} when the scanner gave no name. */
    unnamed: "a threat",
    /** Stands for {scanner} when its name is not known. */
    theScanner: "The scanner",
    missing: "Not scanned: no scanner is set up. Set one in Settings, After download.",
    missingNamed: "Not scanned: {scanner} was not found.",
    failed: "{scanner} could not scan the file: {detail}",
  },
  extract: {
    label: "Extracted",
    done: { one: "{count} file into a new folder.", other: "{count} files into a new folder." },
    skipped: {
      one: "{count} entry was left out: an unsafe name, a link or a duplicate.",
      other: "{count} entries were left out: unsafe names, links or duplicates.",
    },
    deleted: "The archive was deleted afterwards.",
    openFolder: "Open folder",
    failed: "Not extracted: {reason}.",
    errors: {
      encrypted: "the archive is password-protected",
      too_large: "it would expand far beyond its own size (a possible zip bomb)",
      too_many_entries: "it has too many entries",
      corrupt: "the archive is damaged or uses an unsupported method",
      io: "the files could not be written",
    },
  },
  command: {
    label: "Command",
    ok: "{program} finished.",
    failed: "{program} ended with exit code {code}.",
    ended: "{program} was ended before it finished.",
    timedOut: "{program} took longer than the time limit and was stopped.",
    notStarted: "{program} could not be started: {detail}",
    invalid: "The command cannot run: {detail}",
    /** Stands for {program} when its name is not known. */
    theCommand: "The command",
    output: "Output",
  },
  /** The per-download fields in the Add and confirmation dialogs and the details. */
  fields: {
    heading: "After download",
    command: "Run when done",
    /** {command} is the command that runs when the field stays empty. */
    inherited: "Default: {command}",
    nothing: "Nothing",
    hint: "A program and its arguments, such as notepad {path}. {path}, {name}, {folder} and {url} are filled in, each as one argument, without a shell. Not run when the download fails.",
    extract: "Extract the archive",
    extractHint: "Into a new folder next to it, when it completes.",
    save: "Save",
    saved: "Saved. It applies when the download completes.",
  },
  settings: {
    tab: "After download",
    command: "Run a command after each download",
    commandHint:
      "A program and its arguments, such as notepad {path}. {path}, {name}, {folder} and {url} are filled in, each as one argument; nothing goes through a shell, so file names and links cannot add commands. Queues and single downloads can set their own. Not run when a download fails.",
    timeout: "Time limit",
    timeoutHint: "Seconds a command or a scan may run before it is stopped, with everything it started. 5 to 86400.",
    seconds: "s",
    scan: "Scan downloads for viruses",
    scanHintWindows:
      "With Microsoft Defender, after each download completes. A threat marks the download and shows a notification; nothing is deleted or quarantined, and nothing is extracted or run.",
    scanHintOther:
      "With the scanner below, after each download completes. A threat marks the download and shows a notification; nothing is deleted, extracted or run.",
    scanner: "Scanner",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Empty uses Microsoft Defender on Windows. Any other scanner gets the file as {path}, or as its last argument; exit code 0 means clean and 1 a threat, as with ClamAV's clamscan.",
    archives: "Archives",
    archivesHint:
      "Zip, tar, tar.gz, tar.xz and 7z archives are extracted into a new folder next to them when their category says so (Categories) or the download does. Entries that would land outside that folder, links and archives that expand far beyond their size are refused. RAR is not supported; 7-Zip can extract it as a command.",
    deleteArchive: "Delete the archive after extracting it",
    deleteArchiveHint: "Only when every entry was extracted, and after the command ran.",
    /** {name} is the queue's name. */
    queueCommand: "Command after {name} downloads",
    queueCommandUnnamed: "Command after the queue's downloads",
    queueCommandPlaceholder: "Command when done; empty: the one in After download",
    categoryExtract: "Extract archives",
    categoryExtractTitle: "Extract this category's zip, tar and 7z archives after they download",
  },
};
