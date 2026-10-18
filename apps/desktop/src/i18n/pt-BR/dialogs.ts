// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  // "One" and "many" follow whether a single download is selected, not plural rules.
  remove: {
    deleteFile: "Remover e excluir arquivo",
    deleteFiles: "Remover e excluir arquivos",
    deletePartial: "Remover e excluir dados parciais",
    fromList: "Remover da lista",
    doneOne: "Remover da lista mantém o arquivo no disco. Excluir também apaga o próprio arquivo.",
    doneMany: "Remover da lista mantém os arquivos no disco. Excluir também apaga os próprios arquivos.",
    partialOne:
      "Remover da lista mantém o arquivo parcial, para que o mesmo link ainda possa ser retomado depois. Excluir também apaga os dados parciais.",
    partialMany: "Remover da lista mantém no disco os arquivos concluídos e os dados parciais. Excluir apaga ambos.",
  },
  redownload: {
    eyebrow: "Baixar novamente",
    body: "Baixa o arquivo novamente do mesmo link para a mesma pasta, por exemplo, quando ele mudou no servidor ou a cópia no disco está danificada.",
    replace: "Substituir o arquivo existente",
    replaceOn: "O arquivo atual é excluído primeiro, e o novo fica com o nome dele.",
    replaceOff: "Desativado: o arquivo atual permanece, e a nova cópia é salva ao lado dele com um nome disponível.",
    confirm: "Baixar novamente",
    confirmReplace: "Excluir e baixar novamente",
  },
  duplicate: {
    eyebrow: "Já está na lista",
    titleMany: {
      one: "{count} link já está na lista",
      many: "{count} links já estão na lista",
      other: "{count} links já estão na lista",
    },
    body: "Um download com este endereço já está na lista ({state}). O que você deseja fazer?",
    bodyMany: {
      one: "Um download com o mesmo endereço já está na lista. O que você deseja fazer?",
      many: "Downloads com estes endereços já estão na lista. O que você deseja fazer com todos os {count}?",
      other: "Downloads com estes endereços já estão na lista. O que você deseja fazer com todos os {count}?",
    },
    question: "O que fazer com o link",
    copy: "Adicionar novamente com um nome de arquivo numerado",
    overwrite: "Adicionar novamente e substituir o arquivo existente",
    existing: "Mostrar o download existente se estiver concluído; caso contrário, retomá-lo",
    ok: "OK",
  },
};
