# Feature Specification: Backup e Restauração da Base de Dados

**Feature Branch**: `012-db-backup-restore`

**Created**: 2026-07-23

**Status**: Draft

**Input**: User description: "Permitir backup manual da base de dados SQLite (financas.db) para uma pasta escolhida pelo usuário e importação/restauração posterior de um arquivo de backup. Backup: gerar cópia do banco com nome contendo timestamp em pasta selecionada via diálogo. Importar: selecionar arquivo .db de backup e substituir o banco atual (com confirmação, já que sobrescreve dados). 100% local, sem rede. Acesso pela tela de Configurações."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Exportar backup da base para uma pasta (Priority: P1)

O usuário quer guardar uma cópia de segurança de todos os seus dados
financeiros (faturas, contracheques, categorias, lançamentos avulsos).
Na tela de Configurações ele aciona "Fazer backup", escolhe uma pasta no
computador e o app grava ali uma cópia completa da base, com data e hora
no nome do arquivo, para não sobrescrever backups anteriores.

**Why this priority**: Sem backup, uma falha de disco, erro de importação
ou troca de máquina apaga anos de dados que não existem em nenhum outro
lugar (o app é 100% local). É a proteção mínima do usuário e independe da
restauração para entregar valor.

**Independent Test**: Escolher uma pasta, acionar o backup e confirmar que
surge um arquivo `.db` com timestamp no nome, cujo conteúdo abre e reflete
os dados atuais.

**Acceptance Scenarios**:

1. **Given** o app com dados e a tela de Configurações aberta, **When** o
   usuário aciona "Fazer backup" e seleciona uma pasta, **Then** o app grava
   nessa pasta um arquivo de backup com data e hora no nome e exibe
   confirmação com o caminho completo do arquivo gerado.
2. **Given** já existe um backup anterior na mesma pasta, **When** o usuário
   faz um novo backup, **Then** o novo arquivo é criado sem apagar nem
   sobrescrever o anterior (nomes distintos por timestamp).
3. **Given** o usuário cancela o diálogo de escolha de pasta, **When** o
   diálogo fecha, **Then** nenhum arquivo é gravado e nenhum erro é exibido.

---

### User Story 2 - Restaurar/importar a base a partir de um backup (Priority: P2)

O usuário quer recuperar seus dados a partir de um arquivo de backup — após
trocar de máquina, reinstalar o app ou desfazer uma importação equivocada.
Na tela de Configurações ele aciona "Restaurar backup", escolhe um arquivo
de backup, confirma que os dados atuais serão substituídos e o app passa a
mostrar os dados do backup.

**Why this priority**: Completa o ciclo do backup — um backup só tem valor
se puder ser restaurado. Depende conceitualmente da existência de um arquivo
de backup (P1), por isso vem depois.

**Independent Test**: Com um arquivo de backup válido em mãos, acionar a
restauração, confirmar a substituição e verificar que o painel passa a
refletir os dados do backup.

**Acceptance Scenarios**:

1. **Given** um arquivo de backup válido, **When** o usuário aciona "Restaurar
   backup", seleciona o arquivo e confirma a substituição, **Then** a base
   atual é substituída pela do backup e os dados do backup aparecem no app.
2. **Given** o diálogo de confirmação de substituição, **When** o usuário
   cancela, **Then** a base atual permanece intacta e nada é importado.
3. **Given** o usuário seleciona um arquivo que não é uma base válida do app,
   **When** confirma a restauração, **Then** a base atual permanece intacta e
   o app exibe mensagem de erro explicando que o arquivo é inválido.
4. **Given** uma restauração bem-sucedida, **When** ela termina, **Then** o
   app garante que os dados atuais foram preservados em uma cópia de
   segurança automática antes da substituição, permitindo reverter.

---

### Edge Cases

- **Pasta sem permissão de escrita**: o app informa que não conseguiu gravar
  e não deixa o usuário achar que o backup foi feito.
- **Espaço em disco insuficiente**: a gravação falha de forma limpa, sem
  deixar arquivo de backup truncado apresentado como válido.
- **Arquivo de restauração corrompido ou de outra versão do esquema**: a base
  atual não é substituída; erro claro é exibido.
- **App em uso durante a operação**: o backup reflete um estado consistente
  da base; a restauração só troca a base quando pode fazê-lo com segurança.
- **Mesmo segundo em dois backups**: os nomes ainda não colidem (o app evita
  sobrescrever silenciosamente um backup existente).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema DEVE permitir que o usuário faça, sob demanda, um
  backup completo da base de dados a partir da tela de Configurações.
- **FR-002**: O sistema DEVE deixar o usuário escolher, via diálogo do sistema
  operacional, a pasta de destino do backup.
- **FR-003**: O nome do arquivo de backup DEVE conter data e hora, de modo que
  backups sucessivos não sobrescrevam os anteriores.
- **FR-004**: O backup DEVE conter todos os dados do usuário (faturas,
  contracheques, categorias, lançamentos avulsos, configurações), de forma que
  uma restauração recupere o estado por completo.
- **FR-005**: Após o backup, o sistema DEVE confirmar o sucesso e informar o
  caminho completo do arquivo gerado.
- **FR-006**: O sistema DEVE permitir que o usuário restaure/importe a base a
  partir de um arquivo de backup escolhido via diálogo, pela tela de
  Configurações.
- **FR-007**: Antes de substituir a base atual, o sistema DEVE exibir uma
  confirmação explícita avisando que os dados atuais serão substituídos.
- **FR-008**: O sistema DEVE validar o arquivo escolhido para restauração e
  recusar arquivos que não sejam uma base válida do app, sem alterar a base
  atual.
- **FR-009**: Antes de substituir a base durante a restauração, o sistema DEVE
  preservar automaticamente uma cópia de segurança da base atual, permitindo
  reverter em caso de arrependimento ou erro.
- **FR-010**: Se qualquer operação (backup ou restauração) falhar, o sistema
  DEVE manter os dados existentes intactos e exibir mensagem de erro
  compreensível para o usuário.
- **FR-011**: Após uma restauração bem-sucedida, o app DEVE passar a exibir os
  dados restaurados sem exigir procedimentos manuais adicionais do usuário.
- **FR-012**: Todas as operações DEVEM ocorrer localmente, sem qualquer acesso
  à rede.

### Key Entities *(include if feature involves data)*

- **Base de dados**: o conjunto completo dos dados financeiros do usuário,
  fonte única de verdade do app; alvo do backup e da restauração.
- **Arquivo de backup**: cópia da base em um ponto no tempo, identificada por
  data/hora, armazenada numa pasta escolhida pelo usuário.
- **Cópia de segurança automática**: cópia da base atual criada pelo próprio
  app imediatamente antes de uma restauração, para permitir reversão.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O usuário completa um backup em até 30 segundos, do acionamento
  à mensagem de confirmação, para uma base de tamanho típico de uso pessoal.
- **SC-002**: 100% dos backups gerados podem ser restaurados com sucesso,
  reproduzindo integralmente os dados existentes no momento do backup.
- **SC-003**: Nenhuma operação de backup ou restauração que falhe resulta em
  perda ou corrupção dos dados atuais do usuário (a base anterior é sempre
  recuperável).
- **SC-004**: Backups sucessivos na mesma pasta nunca sobrescrevem um backup
  anterior (0% de colisão de nomes em uso normal).
- **SC-005**: O usuário consegue localizar e usar as ações de backup e
  restauração na tela de Configurações sem instrução externa.

## Assumptions

- Operação **manual** sob demanda; backup automático/agendado está fora do
  escopo desta versão.
- O backup é uma cópia integral da base (não um export seletivo por período,
  conta ou categoria).
- Restaurar **substitui** a base atual por completo; mesclar dados de dois
  bancos está fora do escopo.
- O usuário é responsável por onde guarda os arquivos de backup (pen drive,
  nuvem sincronizada, etc.); o app apenas grava no local escolhido.
- Reutiliza a base SQLite existente (`financas.db` em app_data_dir) como fonte
  única de verdade e o mecanismo de diálogo de arquivos já presente no app.
- Público-alvo: usuário único da máquina, sem necessidade de controle de acesso
  entre múltiplos usuários.
