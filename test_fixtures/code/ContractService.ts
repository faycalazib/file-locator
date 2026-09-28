export class ContractService {
  async renew(contract: Contract, months: number) {
    return this.repo.save({ ...contract, months });
  }
}
