# Manual de uso — Finanças

Um guia rápido pra tirar o máximo do app. Tudo acontece no seu computador; nada vai pra nuvem.

> Dica: você não precisa configurar nada antes. Abra, importe um arquivo e o painel se monta sozinho.

---

## Sumário

1. [Primeiros passos](#1-primeiros-passos)
2. [Importar a fatura do cartão](#2-importar-a-fatura-do-cartão)
3. [Importar o contracheque](#3-importar-o-contracheque)
4. [Entender o painel do mês](#4-entender-o-painel-do-mês)
5. [Contas fixas e renda](#5-contas-fixas-e-renda)
6. [Lançamentos avulsos](#6-lançamentos-avulsos)
7. [Categorias e mapeamento](#7-categorias-e-mapeamento)
8. [Ver o ano inteiro](#8-ver-o-ano-inteiro)
9. [Teto do cartão](#9-teto-do-cartão)
10. [Gerar um relatório em PDF](#10-gerar-um-relatório-em-pdf)
11. [Seus dados e backup](#11-seus-dados-e-backup)
12. [Dúvidas comuns](#12-dúvidas-comuns)

---

## 1. Primeiros passos

1. Baixe o instalador na página de **Releases** e instale.
   - **macOS**: se aparecer "desenvolvedor não identificado", clique com o botão direito no app → **Abrir** → **Abrir**. Só na primeira vez.
   - **Windows**: se aparecer o SmartScreen, clique em **Mais informações** → **Executar assim mesmo**.
2. Abra o app. Sem cadastro, sem login.
3. Você cai na tela **Mês**. Ela fica vazia até você importar algo — é o próximo passo.

---

## 2. Importar a fatura do cartão

O app lê a fatura do **cartão BTG** em `.xlsx` (o arquivo que o banco disponibiliza).

1. Clique em **Importar** (topo da tela).
2. Escolha o arquivo `.xlsx` da fatura.
3. Se a fatura estiver **protegida por senha**, o app pede a senha uma vez e guarda no cofre do sistema (Keychain no Mac / Gerenciador de Credenciais no Windows). Não fica salva no banco nem em texto.
4. Pronto: as compras entram já **categorizadas**, com parcelas e estornos reconhecidos.

**Importou o mês errado ou quer atualizar?** Importe de novo — o app substitui a fatura daquele mês em vez de duplicar.

---

## 3. Importar o contracheque

O app lê o **contracheque do SouGov.br** em PDF e separa salário, bônus e descontos.

1. Vá em **Contracheque**.
2. Selecione um ou vários PDFs de uma vez.
3. Confira a prévia e confirme.

O que ele entende sozinho:
- **Salário** × **bônus** (gratificações, férias, 13º e o "Cargo de Direção – CD", tratado como bônus temporário).
- **Descontos**: Imposto de Renda → *Impostos*, GEAP → *Saúde*, FUNPRESP/PSS → *Previdência*.
- **Adiantamentos que se anulam** (quando um valor entra e sai no mesmo mês) — não contam duas vezes.

A partir daí, o **líquido do mês** alimenta o painel.

---

## 4. Entender o painel do mês

A tela **Mês** responde: *para onde foi o dinheiro e sobrou ou faltou?*

- **Indicadores no topo**: receita, despesa total, saldo, líquido do contracheque, descontos e bônus.
- **Composição da despesa**: uma barra dividida em **cartão · fixos · avulsos · descontos da folha**. Bate o olho e você vê o peso de cada bloco.
- **Mapa de gastos (treemap)**: cada retângulo é uma categoria; quanto maior, mais você gastou.
- **Gasto por categoria (casa completa)**: as barras por categoria. **Clique numa categoria** para abrir a lista de despesas dela — com a origem de cada uma (cartão, fixo, avulso, folha).

> Para ver os lançamentos de uma categoria, selecione **um mês** no filtro. Em "Todos os meses" a barra é um total agregado.

Use o filtro de mês no topo para trocar de mês ou ver "Todos os meses".

---

## 5. Contas fixas e renda

Vá em **Fixos & Renda** para cadastrar o que se repete todo mês:

- **Contas fixas** (aluguel, internet, água, energia, ração…). Elas contam automaticamente em todos os meses.
- **Renda extra** (bolsa, aluguel recebido…). O salário vem do contracheque; aqui você adiciona só o que é extra.
- **Despesa esporádica**: dá pra lançar algo que se repete por *N meses* (ex.: uma terapia de 3 meses) sem recadastrar.

---

## 6. Lançamentos avulsos

Gastos ou recebimentos **pontuais** — um freelance que caiu, uma conta que apareceu — não são conta fixa.

1. Na tela **Mês**, clique em **+ Lançamento avulso**.
2. Escolha **débito** ou **crédito**, descrição, valor e categoria.
3. Ele entra no mês selecionado, separado das contas fixas.

Para **editar ou remover**, passe o mouse na lista **Lançamentos avulsos — detalhe** e use ✎ / ✕.

---

## 7. Categorias e mapeamento

O app já classifica as compras sozinho, mas você manda:

- **Ajuste rápido**: mudou de ideia sobre uma transação? Troque a categoria — o app lembra dessa escolha (override) mesmo se você reimportar.
- **Regras por palavra-chave** (tela **Mapeamento**): associe um termo a uma categoria (ex.: "Petz" → *Cachorros*) e ele passa a valer para **todas** as transações que combinam, sempre.
- As regras são **acento-insensíveis** e pegam a raiz da palavra. Ao abrir o app, tudo é recategorizado com as regras atuais (seus ajustes manuais têm prioridade).

---

## 8. Ver o ano inteiro

A tela **Ano** mostra a foto do período.

- **Filtro**: escolha **ano inicial → ano final** e, se quiser, um intervalo de meses.
- **Gráfico**: receita × despesa, mês a mês.
- **Matriz categoria × ano**: cada linha é uma categoria, cada coluna um ano, a cor indica o tamanho do gasto. **Clique numa linha** para marcar/desmarcar a categoria.
- **Evolução por categoria**: as categorias que você marcou viram um gráfico — **uma linha por categoria + uma linha de Total**.
- **Mapa de gastos** e **ranking** do período.

Assim você enxerga hábitos: aquele gasto pequeno e repetido que soma muito no fim do ano.

---

## 9. Teto do cartão

O **teto** responde: *quanto o cartão pode gastar sem furar o orçamento?* É a sua renda menos as contas fixas.

O app mostra **dois cenários**:
- **Renda recorrente**: conta toda a renda que entra todo mês.
- **Só salário**: considera apenas o salário permanente (sem bônus/CD, que são temporários).

Se o gasto do cartão passar do teto, o app sinaliza. Bom para decidir antes da próxima fatura.

---

## 10. Gerar um relatório em PDF

Um resumo pronto pra arquivar ou compartilhar.

1. Na tela **Mês** ou **Ano**, clique em **📄 Relatório**.
2. Confira o resumo em tela cheia (o do Ano respeita o filtro atual).
3. Clique em **Exportar PDF** — o relatório abre no seu navegador; use **Imprimir → Salvar como PDF**.

---

## 11. Seus dados e backup

- Tudo fica num arquivo local no seu computador. **Nada é enviado pra internet.**
- **Backup**: para levar seus dados para outra máquina ou guardar, copie a pasta de dados do app:
  - **macOS**: `~/Library/Application Support/com.financas.app/`
  - **Windows**: `%APPDATA%\com.financas.app\`
- A senha da fatura fica no cofre do sistema, não nesse arquivo.

---

## 12. Dúvidas comuns

**Preciso de internet?** Não. O app funciona 100% offline.

**Meus dados vão para algum servidor?** Não. Nem contas, nem telemetria, nem sincronização.

**A fatura está cifrada — e agora?** Informe a senha quando o app pedir. Ele guarda com segurança e não pergunta de novo.

**Importei duas vezes o mesmo mês.** Sem problema: o app substitui, não duplica.

**A categoria de uma compra ficou errada.** Troque na hora (fica salvo) ou crie uma regra em **Mapeamento** para valer sempre.

**Os números do mês não batem com a barra ao clicar numa categoria.** Selecione **um mês** no filtro — o detalhe é por mês; em "Todos os meses" a barra é um agregado.

**Posso confiar nos valores?** O app usa cálculo exato de dinheiro (sem erros de arredondamento de ponto flutuante).
