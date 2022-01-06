const fs = require('fs')

module.exports = async ({github, context, core}) => {
  const {PR_NUM, REPO_OWNER} = process.env;
    const { data: pullRequest } = await github.rest.pulls.get({
        owner: REPO_OWNER,
        repo: 'cavalcade',
        pull_number: PR_NUM,
        mediaType: {
          format: 'diff'
        }
    });

    console.log(pullRequest);

    const checks = await github.rest.checks.listForRef({
      owner: REPO_OWNER,
      repo: 'cavalcade',
      ref: pullRequest.head.sha,
    });

    console.log(JSON.stringify(checks))

    for (check in checks.data.check_runs) {
      console.log(check)
      const annotations = await github.rest.checks.listAnnotations({
        owner: REPO_OWNER,
        repo: 'cavalcade',
        check: check.id
      });
      console.log(annotations);
    }
    throw "foo";
}