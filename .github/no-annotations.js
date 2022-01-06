module.exports = async ({github, context, core}) => {
  const {SHA} = process.env
  const commit = await github.rest.repos.getCommit({
    owner: context.repo.owner,
    repo: context.repo.repo,
    ref: `${SHA}`
  })
    const { data: pullRequest } = await github.rest.pulls.get({
        owner: 'palfrey',
        repo: 'cavalcade',
        pull_number: 123,
        mediaType: {
          format: 'diff'
        }
    });

    console.log(pullRequest);
}

run();