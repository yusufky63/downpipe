// Small dialogs: removing downloads, and fetching a finished file again.
export const dialogs = {
  /**
   * One download or several. The "one" and "many" texts are picked by
   * whether a single download is selected, not by plural rules, because they
   * say "it" or "them" rather than a number.
   */
  remove: {
    deleteFile: "Remove and delete file",
    deleteFiles: "Remove and delete files",
    deletePartial: "Remove and delete partial data",
    fromList: "Remove from list",
    doneOne: "Removing it from the list keeps the file on disk. Deleting also removes the file itself.",
    doneMany: "Removing them from the list keeps the files on disk. Deleting also removes the files themselves.",
    partialOne: "Removing it from the list keeps the partial file, so the same link can still resume later. Deleting removes the partial data too.",
    partialMany: "Removing them from the list keeps finished files and partial data on disk. Deleting removes both.",
  },
  redownload: {
    eyebrow: "Download again",
    body: "Downloads the file again from the same link into the same folder, for example when it changed on the server or the copy on disk is damaged.",
    replace: "Replace the existing file",
    replaceOn: "The current file is deleted first, and the new one takes its name.",
    replaceOff: "Off: the current file stays, and the new copy is saved next to it under a free name.",
    confirm: "Download again",
    confirmReplace: "Delete and download again",
  },
  /** IDM's question for a link that is already in the list. */
  duplicate: {
    eyebrow: "Already in the list",
    titleMany: { one: "{count} link is already in the list", other: "{count} links are already in the list" },
    body: "A download with this address is already in the list ({state}). What do you want to do?",
    bodyMany: {
      one: "A download with the same address is already in the list. What do you want to do?",
      other: "Downloads with these addresses are already in the list. What do you want to do with all {count}?",
    },
    question: "What to do with the link",
    copy: "Add it again with a numbered file name",
    overwrite: "Add it again and replace the existing file",
    existing: "Show the existing download if it is finished, otherwise resume it",
    ok: "OK",
  },
};
